use anyhow::{ensure,Result};
use serde_json::{Value,json};
use std::collections::HashMap;
use crate::session::Session;

pub fn key(id:&str,group:bool)->String{
    if group {format!("g:{}",id.trim_start_matches("group_").trim_start_matches('g'))}else{format!("p:{id}")}
}
fn number(v:&Value)->Option<i64>{v.as_i64().or_else(||v.as_str()?.parse().ok())}
fn snapshot(v:&Value,now:i64)->Result<HashMap<String,i64>>{
    ensure!(number(&v["error_code"])!=Some(-1)&&number(&v["error_code"]).is_none_or(|n|n==0),"Máy chủ từ chối");
    let data=v.get("data").unwrap_or(v);
    let decoded=data.as_str().and_then(|s|serde_json::from_str::<Value>(s).ok());
    let data=decoded.as_ref().unwrap_or(data);
    let mut result=HashMap::new();
    for (name,group) in [("chatEntries",false),("groupChatEntries",true)] {
        let entries=data[name].as_array().ok_or_else(||anyhow::anyhow!("Danh sách tắt thông báo chưa đầy đủ"))?;
        for e in entries {
            let id=e["id"].as_str().map(str::to_owned).or_else(||e["id"].as_u64().map(|n|n.to_string())).unwrap_or_default();
            if id.is_empty()||id=="0"||number(&e["muteMode"]).unwrap_or(1)!=1{continue;}
            let duration=number(&e["duration"]).unwrap_or(-1);
            if duration==0{continue;}
            let until=if duration<0{-1}else{number(&e["startTime"]).or_else(||number(&e["systemTime"])).unwrap_or(now).saturating_add(duration)};
            if until<0||until>now{result.insert(key(&id,group),until);}
        }
    }
    Ok(result)
}
pub async fn fetch(s:&Session)->Result<HashMap<String,i64>>{
    ensure!(s.login.is_some(),"Chưa đăng nhập");
    let params=s.encoded_params(&json!({"imei":s.imei}))?;
    let url=format!("{}/api/social/profile/getmute?{}&params={params}",s.domains.profile.trim_end_matches('/'),s.common_params());
    let v=s.get(&url).await?;
    snapshot(&s.maybe_decrypt(v,s.session_key_bytes()).await,crate::login::unix_now())
}
pub async fn set(s:&Session,id:&str,group:bool,muted:bool)->Result<()>{
    ensure!(s.login.is_some(),"Chưa đăng nhập");
    let id=if group{id.trim_start_matches("group_").trim_start_matches('g')}else{id};
    let params=s.encoded_params(&json!({"toid":id,"duration":if muted{-1}else{0},"action":if muted{1}else{2},"startTime":crate::login::unix_now(),"muteType":if group{2}else{1},"imei":s.imei}))?;
    let url=format!("{}/api/social/profile/setmute?{}",s.domains.profile.trim_end_matches('/'),s.common_params());
    let v=s.post_form(&url,format!("params={params}")).await?;
    let v=s.maybe_decrypt(v,s.session_key_bytes()).await;
    ensure!(number(&v["error_code"])==Some(0),"Máy chủ chưa xác nhận thay đổi thông báo");
    Ok(())
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn snapshot_rejects_incomplete_and_keeps_groups_distinct(){
        assert!(snapshot(&json!({"chatEntries":[]}),100).is_err());
        let m=snapshot(&json!({"chatEntries":[{"id":"123","duration":-1}],"groupChatEntries":[{"id":"123","duration":30,"startTime":90}]}),100).unwrap();
        assert_eq!(m["p:123"],-1);assert_eq!(m["g:123"],120);
    }
}
