//! Android-to-desktop sync-v2. Transfer keys live in an isolated pipe-connected helper.
//! The public proprietary runtime is fetched at runtime, hash-pinned, never redistributed.
use anyhow::{anyhow,bail,ensure,Context,Result};
use base64::{Engine,engine::general_purpose::STANDARD as B64};
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use std::{collections::{HashMap,HashSet},path::PathBuf,process::Stdio,time::Duration};
use tokio::{io::{AsyncBufReadExt,AsyncWriteExt,BufReader},process::{Child,ChildStdin,ChildStdout},sync::{broadcast,mpsc,watch}};
use crate::{chat::{self,Thread},session::Session,socket::{SocketCommand,WsEvent}};

const WORKER_HASH:&str="b650e02c390f27b6e11e879c0475bfcdbeaa3211368772a5028fd20772fb8dec";
const WASM_HASH:&str="3a868037a32739a4fcb3c22425150745613006f9dbc561f30fd705effaed55fa";
const BRIDGE:&str=include_str!("../scripts/mobile_sync_bridge.cjs");

pub enum Update {Status(String),Threads(Vec<Thread>)}
struct Bridge {child:Child,input:ChildStdin,output:BufReader<ChildStdout>}
impl Bridge {
    async fn start(s:&Session)->Result<Self> {
        let root=PathBuf::from("phone_sync_runtime");std::fs::create_dir_all(&root)?;
        #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;std::fs::set_permissions(&root,std::fs::Permissions::from_mode(0o700))?;}
        for (name,url,hash,max) in [
            ("worker.js","https://zalo-chat-static.zadn.vn/v1/sync-v2-worker.1b2d86e9567f6be024ce.js",WORKER_HASH,8*1024*1024usize),
            ("libzproto.wasm","https://chat.zalo.me/libs/libzproto_wasm_bg.ecc0555ba9d0f8867a4699ce943ec745.wasm",WASM_HASH,2*1024*1024usize)] {
            let path=root.join(name);
            if std::fs::read(&path).ok().is_some_and(|data|hash_bytes(&data)==hash) {continue;}
            let mut response=crate::inline_video::media_response(&s.http,url).await?;let mut data=Vec::new();
            while let Some(chunk)=response.chunk().await? {ensure!(data.len()+chunk.len()<=max,"Thành phần đồng bộ quá lớn");data.extend_from_slice(&chunk);}
            ensure!(hash_bytes(&data)==hash,"Thành phần đồng bộ đã thay đổi; cần cập nhật ứng dụng");
            crate::storage::atomic_write(&path,&data)?;
        }
        let script=root.join("bridge.cjs");crate::storage::atomic_write(&script,BRIDGE.as_bytes())?;
        let root=std::fs::canonicalize(root)?;
        let mut command=tokio::process::Command::new(std::env::var_os("CTDOZLITE_NODE").unwrap_or_else(||"node".into()));
        command.arg(std::fs::canonicalize(script)?).arg(root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
        #[cfg(target_os="windows")]command.creation_flags(0x08000000);
        let mut child=command.spawn().context("Cần Node.js 24 trở lên cho bộ giải mã đồng bộ điện thoại")?;
        let input=child.stdin.take().unwrap();let output=BufReader::new(child.stdout.take().unwrap());
        Ok(Self{child,input,output})
    }
    async fn request(&mut self,command:Value)->Result<Value> {
        let mut bytes=serde_json::to_vec(&command)?;bytes.push(b'\n');self.input.write_all(&bytes).await?;self.input.flush().await?;
        let result=tokio::time::timeout(Duration::from_secs(60),async {
            let mut line=Vec::new();
            // Bound the helper response before deserializing user history.
            loop {let available=self.output.fill_buf().await?;ensure!(!available.is_empty(),"Bộ giải mã đã dừng");let count=available.iter().position(|b|*b==b'\n').map(|n|n+1).unwrap_or(available.len());ensure!(line.len()+count<=64*1024*1024,"Dữ liệu đồng bộ quá lớn");let complete=available[count-1]==b'\n';line.extend_from_slice(&available[..count]);self.output.consume(count);if complete{break;}}
            let response:Value=serde_json::from_slice(&line)?;
            ensure!(response["ok"].as_bool()==Some(true),"Không giải mã được lô dữ liệu từ điện thoại");
            Ok::<_,anyhow::Error>(response["result"].clone())
        }).await.context("Bộ giải mã không phản hồi")??;Ok(result)
    }
}
impl Drop for Bridge {fn drop(&mut self){let _=self.child.start_kill();}}
fn hash_bytes(bytes:&[u8])->String {format!("{:x}",Sha256::digest(bytes))}
fn random_id()->Result<String> {use aes_gcm::aead::rand_core::RngCore;let mut bytes=[0u8;16];aes_gcm::aead::OsRng.fill_bytes(&mut bytes);Ok(bytes.iter().map(|b|format!("{b:02x}")).collect())}
fn text(value:&Value)->Option<String> {value.as_str().map(str::to_owned).or_else(||value.as_u64().map(|v|v.to_string()))}
fn yes(value:Option<&Value>)->bool {value.is_some_and(|v|v.as_bool()==Some(true)||v.as_i64()==Some(1))}

/// Only sync-v2 controls are extracted; a malformed nested body cannot recurse indefinitely.
pub fn controls(value:&Value)->Vec<Value> {
    fn walk(value:&Value,depth:usize,output:&mut Vec<Value>) {
        if depth>12||output.len()>=4096 {return;}
        if let Some(act)=value.get("act").and_then(Value::as_str) {
            if matches!(act,"transfer_after_login"|"transfer_status"|"upload_batch"|"transfer_error") {let mut control=value.clone();if let Some(raw)=control["data"].as_str(){if let Ok(data)=serde_json::from_str::<Value>(raw){control["data"]=data;}}output.push(control);return;}
        }
        match value {Value::Array(a)=>for v in a {walk(v,depth+1,output)},Value::Object(o)=>for v in o.values(){walk(v,depth+1,output)},Value::String(s) if s.len()<1024*1024&&(s.starts_with('{')||s.starts_with('['))=>{if let Ok(v)=serde_json::from_str::<Value>(s){walk(&v,depth+1,output)}},_=>{}}
    }
    let mut output=Vec::new();walk(value,0,&mut output);output
}

#[derive(Default)]
struct Mapping {threads:HashMap<String,Thread>,users:HashMap<String,String>,names:HashMap<String,String>,own_global:String}
impl Mapping {
    async fn load(s:&Session)->Result<Self> {
        let mut threads=chat::load_threads_cache();
        if threads.is_empty(){threads=chat::get_friends(s).await?;}
        let mut mapping=Self::default();
        let group_ids:Vec<String>=threads.iter().filter(|t|t.is_group).map(|t|t.id.clone()).collect();
        for chunk in group_ids.chunks(25) {
            let map:serde_json::Map<String,Value>=chunk.iter().map(|id|(id.clone(),json!(0))).collect();
            let params=s.encoded_params(&json!({"gridVerMap":serde_json::to_string(&map)?}))?;
            let response=s.post_form(&format!("{}/api/group/getmg-v2?{}",s.domains.group,s.common_params()),format!("params={params}")).await?;
            let decoded=s.maybe_decrypt(response,s.session_key_bytes()).await;let data=chat::response_payload(&decoded);
            if let Some(profiles)=data["gridInfoMap"].as_object() {for (id,profile) in profiles {
                if let (Some(global),Some(thread))=(text(&profile["globalId"]),threads.iter().find(|t|t.id==*id)) {mapping.threads.insert(format!("group/{global}"),thread.clone());}
            }}
        }
        let mut user_ids:Vec<String>=threads.iter().filter(|t|!t.is_group).map(|t|t.id.clone()).collect();
        if let Some(login)=s.login.as_ref(){user_ids.push(login.uin.clone());}
        for chunk in user_ids.chunks(100) {
            let data=gid_request(s,"encrypt",json!({"noiseUids":chunk})).await?;
            if let Some(ids)=data.as_object(){for (local,global) in ids {if let Some(global)=text(global) {
                mapping.users.insert(global.clone(),local.clone());
                if s.login.as_ref().is_some_and(|l|l.uin==*local){mapping.own_global=global.clone();}
                if let Some(thread)=threads.iter().find(|t|t.id==*local){mapping.names.insert(global.clone(),thread.name.clone());mapping.threads.insert(format!("oneone/{global}"),thread.clone());}
            }}}
        }
        ensure!(!mapping.own_global.is_empty(),"Chưa ánh xạ được tài khoản để xác định tin gửi/nhận");
        Ok(mapping)
    }
    async fn resolve_users(&mut self,s:&Session,globals:&[String])->Result<()> {
        let mut missing:Vec<String>=globals.iter().filter(|id|!id.is_empty()&&id.as_str()!="0"&&!self.users.contains_key(*id)).cloned().collect();
        missing.sort();missing.dedup();
        for chunk in missing.chunks(100) {
            let data=gid_request(s,"decrypt",json!({"globalUids":serde_json::to_string(chunk)?})).await?;
            if let Some(ids)=data.as_object(){for (global,local) in ids {if let Some(local)=text(local){self.users.insert(global.clone(),local);}}}
        }
        Ok(())
    }
    async fn add_conversations(&mut self,s:&Session,conversations:&[Value])->Result<Vec<String>> {
        let users:Vec<String>=conversations.iter().filter(|c|c["convType"].as_u64()==Some(1)).filter_map(|c|text(&c["convId"])).collect();self.resolve_users(s,&users).await?;
        let mut partitions=Vec::new();
        for conversation in conversations {
            let Some(id)=text(&conversation["convId"]) else{continue;};
            let group=conversation["convType"].as_u64()==Some(2);let partition=format!("{}/{id}",if group{"group"}else{"oneone"});
            if !group&&!self.threads.contains_key(&partition) {
                if let Some(local)=self.users.get(&id) {self.threads.insert(partition.clone(),Thread{id:local.clone(),name:format!("Người dùng {local}"),is_group:false,preview:String::new(),time:String::new(),avatar:None,messages:Vec::new()});}
            }
            if self.threads.contains_key(&partition){partitions.push(partition);}
        }
        partitions.sort();partitions.dedup();Ok(partitions)
    }
    async fn import(&mut self,s:&Session,chunks:&[Value])->Result<Vec<Thread>> {
        let senders:Vec<String>=chunks.iter().flat_map(|c|c["partitionsList"].as_array().into_iter().flatten()).flat_map(|p|p["messagesList"].as_array().into_iter().flatten()).filter_map(|m|text(&m["senderId"])).collect();
        self.resolve_users(s,&senders).await?;
        let mut changed=Vec::new();
        for partition in chunks.iter().flat_map(|c|c["partitionsList"].as_array().into_iter().flatten()) {
            let Some(id)=partition["id"].as_str() else{continue;};let Some(mut thread)=self.threads.get(id).cloned() else{continue;};thread.messages.clear();
            for message in partition["messagesList"].as_array().into_iter().flatten() {
                if let Some(raw)=wire_message(message,&self.own_global,&self.users,&self.names) {
                    if let Some(message)=chat::parse_chat_message(&raw){thread.messages.push(message);}
                }
            }
            if thread.messages.is_empty(){continue;}
            chat::normalize_thread_times(std::slice::from_mut(&mut thread));
            if let Some(latest)=thread.messages.iter().max_by_key(|m|m.ts){thread.preview=latest.text.replace('\n'," ");}
            changed.push(thread);
        }
        if !changed.is_empty() {
            chat::resolve_thread_names(s,&mut changed).await;
            let mut store=crate::store::MessageStore::open("zalo_history.db")?;
            crate::store::persist_threads(&mut store,&changed)?;
        }
        Ok(changed)
    }
}
async fn gid_request(s:&Session,operation:&str,body:Value)->Result<Value> {
    let params=s.encoded_params(&body)?;let response=s.get(&format!("{}/api/gid/{operation}?{}&params={params}",s.domains.profile,s.common_params())).await?;
    let mut data=chat::response_payload(&s.maybe_decrypt(response,s.session_key_bytes()).await);
    if data.get("data").is_some(){data=chat::response_payload(&data);}
    Ok(data)
}
fn wire_message(message:&Value,own:&str,users:&HashMap<String,String>,names:&HashMap<String,String>)->Option<Value> {
    let global=text(&message["globalId"])?;if global=="0"||global.is_empty(){return None;}
    let sender=text(&message["senderId"])?;let outgoing=sender==own;
    let mut content=message["content"].clone();
    if let Some(attachment)=message.pointer("/meta/attachsList/0") {content=attachment.clone();if content["description"].as_str().is_none_or(str::is_empty)&&message["content"].as_str().is_some_and(|s|!s.is_empty()){content["description"]=message["content"].clone();}}
    let kind=match message["msgType"].as_u64().unwrap_or(u64::MAX) {
        0=>"chat.text",2|3|4|31=>"chat.photo",6=>"chat.voice",10=>"chat.sticker",12=>"chat.contact",18=>"chat.location",19=>"chat.video",22=>"chat.file",36=>"chat.undo",_=>"chat.recommended",
    };
    Some(json!({"msgId":global,"cliMsgId":message["clientId"],"ts":message["timestamp"],"msgType":kind,"content":content,
        "uidFrom":if outgoing{"0".to_string()}else{users.get(&sender).cloned().unwrap_or_else(||sender.clone())},"dName":if outgoing{"Tôi".to_string()}else{names.get(&sender).cloned().unwrap_or_else(||"Người dùng".into())},"isOutgoing":outgoing,
        "quote":message.pointer("/meta/quote").cloned(),"mentions":message.pointer("/meta/mentionsList").cloned(),"property":message.pointer("/meta/property").cloned(),"ttl":message.pointer("/meta/ttl").cloned()}))
}

pub async fn run(s:Session,outbound:mpsc::UnboundedSender<SocketCommand>,mut events:broadcast::Receiver<WsEvent>,mut cancel:watch::Receiver<bool>,mut notify:impl FnMut(Update)+Send)->Result<String> {
    ensure!(s.login.is_some(),"Cần đăng nhập trước khi đồng bộ điện thoại");
    notify(Update::Status("Đang chuẩn bị bộ giải mã và danh sách hội thoại Android…".into()));
    let mut bridge=Bridge::start(&s).await?;
    let mut mapping=Mapping::load(&s).await?;let mut index=0u32;
    notify(Update::Status("Mở Zalo trên Android và xác nhận yêu cầu đồng bộ nếu điện thoại hỏi.".into()));
    let metadata=receive(&s,&outbound,&mut events,&mut cancel,&mut bridge,"conversation",vec![json!({"partition":"","from":0,"to":9007199254740991u64,"limit":2147483647})],&mut index,&mut mapping,&mut notify).await?;
    let conversations:Vec<Value>=metadata.iter().flat_map(|c|c["conversationsList"].as_array().into_iter().flatten()).cloned().collect();
    let partitions=mapping.add_conversations(&s,&conversations).await?;let skipped=conversations.len().saturating_sub(partitions.len());
    ensure!(!partitions.is_empty()||conversations.is_empty(),"Không ánh xạ được các hội thoại điện thoại sang tài khoản hiện tại");
    for (batch,partitions) in partitions.chunks(25).enumerate() {
        if *cancel.borrow(){bail!("Đã dừng đồng bộ; các tin đã nhận được giữ lại");}
        notify(Update::Status(format!("Đang nhận lịch sử Android • nhóm hội thoại {}",batch+1)));
        let queries=partitions.iter().map(|partition|json!({"partition":partition,"from":0,"to":9007199254740991u64,"limit":2147483647})).collect();
        receive(&s,&outbound,&mut events,&mut cancel,&mut bridge,"message",queries,&mut index,&mut mapping,&mut notify).await?;
    }
    Ok(format!("Đã nhận lịch sử từ Android; {skipped} hội thoại không còn ánh xạ trên tài khoản nên chưa nhập được."))
}

struct TransferGuard {outbound:mpsc::UnboundedSender<SocketCommand>,sync_id:String,success:bool}
impl Drop for TransferGuard {fn drop(&mut self){let _=self.outbound.send(SocketCommand{cmd:591,data:json!({"reqId":format!("dispose_{}",crate::login::unix_now()),"data":{"syncId":self.sync_id,"toDevice":0,"reason":if self.success{1}else{0}}})});}}

#[allow(clippy::too_many_arguments)]
async fn receive(s:&Session,outbound:&mpsc::UnboundedSender<SocketCommand>,events:&mut broadcast::Receiver<WsEvent>,cancel:&mut watch::Receiver<bool>,bridge:&mut Bridge,kind:&str,queries:Vec<Value>,index:&mut u32,mapping:&mut Mapping,notify:&mut (impl FnMut(Update)+Send))->Result<Vec<Value>> {
    let sync_id=random_id()?;let req_id=format!("phone_{}_{}",crate::login::unix_now(),index);
    let keys=bridge.request(json!({"op":"session","syncId":sync_id})).await?;
    let request=json!({"syncId":sync_id,"syncType":0,"ek":keys["ek"],"ik":keys["ik"],"toDevice":0,"tempKey":"","deviceName":"ctdozlite Linux","req":{"type":kind,"priority":1,"queries":queries,"batchSize":2000},"ver":1,"ussidx":*index});*index+=1;
    let mut guard=TransferGuard{outbound:outbound.clone(),sync_id:sync_id.clone(),success:false};
    outbound.send(SocketCommand{cmd:590,data:json!({"reqId":req_id,"data":request})}).map_err(|_|anyhow!("Kết nối đồng bộ đã đóng"))?;
    let result=async {
        let mut pending=Vec::<Value>::new();let mut queued=HashSet::new();let mut seen=HashSet::new();let mut expected=None;let mut has_metadata=false;let mut chunks=Vec::new();let mut message_count=0usize;
        let mut last_event=tokio::time::Instant::now();let mut wakeup=tokio::time::interval(Duration::from_secs(30));wakeup.tick().await;
        loop {
            if *cancel.borrow(){bail!("Đã dừng đồng bộ; các tin đã nhận được giữ lại");}
            let event=tokio::select! {
                changed=cancel.changed()=>{let _=changed;bail!("Đã dừng đồng bộ; các tin đã nhận được giữ lại");},
                _=tokio::time::sleep_until(last_event+Duration::from_secs(180))=>bail!("Chưa nhận dữ liệu từ điện thoại. Mở Zalo Android, giữ mạng hoạt động rồi thử đồng bộ lại."),
                _=wakeup.tick()=>{let _=outbound.send(SocketCommand{cmd:592,data:json!({"reqId":format!("wake_{}",crate::login::unix_now()),"syncId":sync_id,"toDevice":0})});continue;},
                event=events.recv()=>event.map_err(|_|anyhow!("Bị gián đoạn nhận dữ liệu đồng bộ; hãy thử lại"))?,
            };
            if event.cmd==590 && event.payload.pointer("/data/reqId").and_then(Value::as_str)==Some(&req_id) {
                let code=event.payload["error_code"].as_i64().unwrap_or(-1);
                ensure!(code==0,"Máy chủ từ chối yêu cầu đồng bộ điện thoại (mã {code})");
                notify(Update::Status("Đã gửi yêu cầu. Hãy mở Zalo Android và cho phép đồng bộ nếu có thông báo.".into()));
            }
            for control in controls(&event.payload) {
                let data=&control["data"];let act=control["act"].as_str().unwrap_or("");
                if data["syncId"].as_str()!=Some(&sync_id){continue;}
                last_event=tokio::time::Instant::now();
                match act {
                    "transfer_status"=>match data["status"].as_i64().unwrap_or(0) {
                        3=>notify(Update::Status("Đang chờ bạn xác nhận đồng bộ trên Zalo Android…".into())),
                        4|5=>notify(Update::Status("Android đã cho phép; đang chờ dữ liệu…".into())),
                        6|7=>bail!("Yêu cầu đồng bộ đã bị từ chối hoặc hủy trên điện thoại"),
                        8=>bail!("Điện thoại đang bận với phiên đồng bộ khác; hãy thử lại sau"),
                        9=>bail!("Điện thoại không đủ dung lượng để chuẩn bị lịch sử"),
                        10=>bail!("Zalo trên điện thoại đã đăng xuất"),_=>{},
                    },
                    "transfer_error"=>bail!("Điện thoại báo lỗi chuyển lịch sử; các tin đã nhận được giữ lại"),
                    "upload_batch"=>{
                        let code=data["err"].as_i64().unwrap_or(0);ensure!(code==0,"Điện thoại không tạo được lô lịch sử (mã {code})");
                        let idx=data["idx"].as_u64().ok_or_else(||anyhow!("Lô lịch sử thiếu số thứ tự"))?;
                        if yes(data.get("isLast")){expected=Some(idx+1);}
                        if queued.insert(idx) {ensure!(pending.len()<4096,"Quá nhiều lô chờ giải mã");pending.push(data.clone());}
                    },_=>{},
                }
            }
            loop {
                let next=pending.iter().position(|batch|batch["batchType"].as_u64()==Some(2)||has_metadata||batch["msgUrl"].as_str().is_none_or(str::is_empty));
                let Some(next)=next else{break;};let batch=pending.remove(next);let idx=batch["idx"].as_u64().unwrap();
                if let Some(url)=batch["msgUrl"].as_str().filter(|url|!url.is_empty()) {
                    let mut response=crate::inline_video::media_response(&s.http,url).await?;let mut bytes=Vec::new();
                    ensure!(response.content_length().unwrap_or(0)<=32*1024*1024,"Lô lịch sử quá lớn");
                    while let Some(chunk)=response.chunk().await?{ensure!(bytes.len()+chunk.len()<=32*1024*1024,"Lô lịch sử quá lớn");bytes.extend_from_slice(&chunk);}
                    let decoded=bridge.request(json!({"op":"decode","syncId":sync_id,"kind":kind,"batchType":batch["batchType"],"bytes":B64.encode(&bytes)})).await?;
                    if batch["batchType"].as_u64()==Some(2){has_metadata=true;}
                    let batch_chunks=decoded["chunks"].as_array().cloned().unwrap_or_default();
                    if kind=="conversation" {chunks.extend(batch_chunks);} else {
                        let changed=mapping.import(s,&batch_chunks).await?;message_count+=changed.iter().map(|t|t.messages.len()).sum::<usize>();
                        if !changed.is_empty(){notify(Update::Threads(changed));}
                        notify(Update::Status(format!("Đang lưu lịch sử Android • {message_count} tin trong đợt hiện tại")));
                    }
                }
                seen.insert(idx);
            }
            if expected.is_some_and(|total|total<=4096&&(0..total).all(|idx|seen.contains(&idx))){break;}
        }
        Ok::<_,anyhow::Error>(chunks)
    }.await;
    guard.success=result.is_ok();
    let _=bridge.request(json!({"op":"release","syncId":sync_id})).await;
    result
}

/// Read-only diagnostics, without printing account IDs, names, keys or message contents.
pub async fn diagnose(s:&Session)->Result<(usize,usize)> {let mapping=Mapping::load(s).await?;Ok((mapping.threads.len(),mapping.users.len()))}
pub async fn bridge_self_test(s:&Session)->Result<()> {let mut bridge=Bridge::start(s).await?;let result=bridge.request(json!({"op":"self_test"})).await?;ensure!(result["passed"]==true,"Bridge test failed");Ok(())}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn extracts_only_sync_controls_and_keeps_session_ids() {
        let events=controls(&json!({"data":{"ctrls":[{"act":"upload_batch","data":{"syncId":"a","idx":0}},{"act":"other","data":{}},{"nested":"{\"act\":\"transfer_status\",\"data\":{\"syncId\":\"a\",\"status\":4}}"}]}}));
        assert_eq!(events.len(),2);assert_eq!(events[0]["data"]["syncId"],"a");
    }
    #[test]fn maps_phone_attachments_without_losing_ids_or_direction() {
        let raw=wire_message(&json!({"globalId":"9007199254740991","clientId":1700000000000u64,"timestamp":1700000000000u64,"msgType":3,"senderId":"own","content":"Caption","meta":{"attachsList":[{"href":"https://photo.zdn.vn/p.jpg","thumb":"https://photo.zdn.vn/t.jpg"}]}}),"own",&HashMap::new(),&HashMap::new()).unwrap();
        let message=chat::parse_chat_message(&raw).unwrap();assert_eq!(message.id,"9007199254740991");assert!(message.is_outgoing);assert_eq!(message.link_src.as_deref(),Some("Hình ảnh Zalo"));assert_eq!(message.text,"Caption");
    }
}

pub async fn diagnose_shape(s:&Session)->Result<Value> {
 let own=s.login.as_ref().map(|l|l.uin.clone()).unwrap_or_default();
 let data=gid_request(s,"encrypt",json!({"noiseUids":[own.clone()]})).await?;
 let shape=|v:&Value|if v.is_object(){"object"}else if v.is_array(){"array"}else if v.is_string(){"string"}else if v.is_null(){"null"}else{"other"};
 Ok(json!({"cached_threads":chat::load_threads_cache().len(),"own_id_empty":own.is_empty(),"own_id_is_zero":own=="0","own_id_length":own.len(),"gid_type":shape(&data),"gid_items":data.as_object().map(|v|v.len()),"gid_has_own_key":data.get(&own).is_some(),"gid_first_value_type":data.as_object().and_then(|v|v.values().next()).map(shape),"gid_known_fields":(["data","result","uids","noiseUids","globalUids","globalId"].into_iter().filter(|k|data.get(*k).is_some()).collect::<Vec<_>>())}))
}
