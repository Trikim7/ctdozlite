'use strict';
// Original IPC adapter. Official Zalo runtime assets are downloaded separately and hash checked.
// No network API is exposed to the VM; the worker bootstrap is never executed.
const fs=require('node:fs'),vm=require('node:vm'),crypto=require('node:crypto'),zlib=require('node:zlib'),readline=require('node:readline');
const path=require('node:path'),assert=require('node:assert/strict');
const root=process.argv[2];
function asset(name,digest){const bytes=fs.readFileSync(path.join(root,name));assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'),digest,'Runtime asset checksum mismatch');return bytes;}
const source=asset('worker.js','b650e02c390f27b6e11e879c0475bfcdbeaa3211368772a5028fd20772fb8dec').toString('utf8');
const wasm=asset('libzproto.wasm','3a868037a32739a4fcb3c22425150745613006f9dbc561f30fd705effaed55fa');
const start=source.indexOf('}({')+2,end=source.lastIndexOf('});\n//# sourceMappingURL')+1;
assert(start>1&&end>start,'Unexpected worker format');
const sandbox={TextEncoder,TextDecoder,WebAssembly,Uint8Array,ArrayBuffer,DataView,crypto:crypto.webcrypto,setTimeout,clearTimeout,Buffer};sandbox.globalThis=sandbox;sandbox.self=sandbox;
const context=vm.createContext(sandbox),modules=vm.runInContext('('+source.slice(start,end)+')',context,{timeout:3000}),cache={};
function load(id){if(cache[id])return cache[id].exports;assert(modules[id],'Missing protobuf module');const mod=cache[id]={exports:{}};modules[id].call(mod.exports,mod,mod.exports,load);return mod.exports;}
load.d=(o,k,f)=>Object.defineProperty(o,k,{get:f,enumerable:true});load.r=o=>Object.defineProperty(o,'__esModule',{value:true});load.o=(o,k)=>Object.prototype.hasOwnProperty.call(o,k);load.n=o=>{const f=o&&o.__esModule?()=>o.default:()=>o;load.d(f,'a',f);return f};load.g=sandbox;load.p='';
const sdk={},sdkLoader=id=>{if(id==='3UD+')return m=>m;if(id==='yLpj')return sandbox;throw Error('Unexpected crypto import')};sdkLoader.d=load.d;modules.Vox7({},sdk,sdkLoader);
const conversation=load('Ho1Y'),message=load('1y5R'),sessions=new Map();let identity;
function frames(bytes){const result=[];for(let at=0;at<bytes.length;){assert(at+4<=bytes.length,'Incomplete chunk header');const length=bytes.readUInt32LE(at);at+=4;assert(length>0&&length<=32*1024*1024&&at+length<=bytes.length,'Invalid chunk length');result.push(bytes.subarray(at,at+length));at+=length;}return result;}
function decode(command){const session=sessions.get(command.syncId);assert(session,'Unknown session');const bytes=Buffer.from(command.bytes,'base64');assert(bytes.length<=32*1024*1024,'Batch too large');const chunks=[];
 for(const encrypted of frames(bytes)) {
  let result;
  if(command.batchType===2){result=sdk.j(encrypted,identity.publicKey,identity.privateKey,session.ek.privateKey);if(session.record)session.record.fill(0);session.record=result.sessionRecord.slice();}
  else {assert(command.batchType===1&&session.record,'Missing encrypted metadata');result=sdk.i(encrypted,session.record);}
  const plain=Buffer.from(result.plaintext);result.free();
  const decompressed=zlib.zstdDecompressSync(plain,{maxOutputLength:64*1024*1024});plain.fill(0);
  chunks.push((command.kind==='conversation'?conversation.SyncChunk:message.SyncChunk).deserializeBinary(decompressed).toObject());decompressed.fill(0);
 }
 return {chunks};
}
async function dispatch(command){switch(command.op){
 case 'session':{assert(sessions.size<512,'Too many sessions');const ek=sdk.e();sessions.set(command.syncId,{ek,record:null});return {ik:Buffer.from(identity.publicKey).toString('base64'),ek:Buffer.from(ek.publicKey).toString('base64')};}
 case 'decode':return decode(command);
 case 'release':{const session=sessions.get(command.syncId);if(session){session.ek.free();if(session.record)session.record.fill(0);sessions.delete(command.syncId);}return {};}
 case 'self_test':return selfTest();
 default:throw Error('Unknown operation');
}}
function framed(bytes){const head=Buffer.alloc(4);head.writeUInt32LE(bytes.length);return Buffer.concat([head,Buffer.from(bytes)]);}
function selfTest(){const sender=sdk.e(),ephemeral=sdk.e();const syncId='self-test';sessions.set(syncId,{ek:ephemeral,record:null});const cv=new conversation.Conversation();cv.setConvId('synthetic');cv.setConvType(2);const chunk=new conversation.SyncChunk();chunk.setConversationsList([cv]);const encrypted=sdk.h(zlib.zstdCompressSync(Buffer.from(chunk.serializeBinary())),sender.publicKey,sender.privateKey,identity.publicKey,ephemeral.publicKey);const a=decode({syncId,kind:'conversation',batchType:2,bytes:framed(encrypted.ciphertext).toString('base64')});assert.equal(a.chunks[0].conversationsList[0].convId,'synthetic');const msg=new message.Message();msg.setContent('Xin chào tiếng Việt');msg.setGlobalId('9007199254740991');const partition=new message.Partition();partition.setId('group/synthetic');partition.setMessagesList([msg]);const messages=new message.SyncChunk();messages.setPartitionsList([partition]);const payload=sdk.k(zlib.zstdCompressSync(Buffer.from(messages.serializeBinary())),encrypted.sessionRecord);const b=decode({syncId,kind:'message',batchType:1,bytes:framed(payload.ciphertext).toString('base64')});assert.equal(b.chunks[0].partitionsList[0].messagesList[0].content,'Xin chào tiếng Việt');assert.equal(b.chunks[0].partitionsList[0].messagesList[0].globalId,'9007199254740991');const broken=Buffer.from(payload.ciphertext);broken[broken.length-1]^=1;assert.throws(()=>decode({syncId,kind:'message',batchType:1,bytes:framed(broken).toString('base64')}));assert.throws(()=>frames(Buffer.from([255,255,255,255])));sender.free();encrypted.free();payload.free();ephemeral.free();sessions.get(syncId).record.fill(0);sessions.delete(syncId);return {passed:true};}
(async()=>{await sdk.b(wasm);identity=sdk.e();
 const reader=readline.createInterface({input:process.stdin,crlfDelay:Infinity});
 for await(const line of reader){if(line.length>48*1024*1024)throw Error('IPC input too large');try{const result=await dispatch(JSON.parse(line));process.stdout.write(JSON.stringify({ok:true,result})+'\n');}catch(_){process.stdout.write(JSON.stringify({ok:false,error:'Cannot decode this transfer batch'})+'\n');}}
 for(const session of sessions.values()){session.ek.free();if(session.record)session.record.fill(0);}identity.free();
})().catch(()=>{process.exitCode=1});
