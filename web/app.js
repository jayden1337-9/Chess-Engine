import { Chess } from "https://cdn.jsdelivr.net/npm/chess.js@1.4.0/+esm";
const $ = id => document.getElementById(id);
const game = new Chess();
const boardEl = $("board");
const glyph = {wk:"♔",wq:"♕",wr:"♖",wb:"♗",wn:"♘",wp:"♙",bk:"♚",bq:"♛",br:"♜",bb:"♝",bn:"♞",bp:"♟"};
let selected = null, legal = [], flipped = false, mode = "local", socket = null, myColor = null, room = null, waiting = false, onlineHistory = [];
const files = "abcdefgh";
function render(){
  boardEl.innerHTML = "";
  const ranks = flipped ? [1,2,3,4,5,6,7,8] : [8,7,6,5,4,3,2,1];
  const cols = flipped ? [...files].reverse() : [...files];
  for(const rank of ranks) for(const file of cols){
    const sq = file+rank, piece = game.get(sq), btn = document.createElement("button");
    btn.className = "square "+((files.indexOf(file)+rank)%2===0?"light":"dark");
    if(selected===sq) btn.classList.add("selected");
    const moves = legal.filter(m=>m.to===sq);
    if(moves.length) btn.classList.add(piece?"capture":"legal");
    if(piece){const span=document.createElement("span");span.className="piece";span.textContent=glyph[piece.color+piece.type];btn.append(span);}
    if((!flipped&&file==="a")||(flipped&&file==="h")){const c=document.createElement("span");c.className="coord";c.textContent=rank;btn.append(c);}
    btn.addEventListener("click",()=>tap(sq));boardEl.append(btn);
  }
  $("turn").textContent = game.isCheckmate() ? "Checkmate" : game.isDraw() ? "Draw" : (game.turn()==="w"?"White":"Black")+" to move";
  $("move-count").textContent = game.history().length;
  const hist=game.history({verbose:true});$("history").innerHTML="";
  if(!hist.length){const li=document.createElement("li");li.className="muted";li.textContent="No moves yet";$("history").append(li);}
  for(let i=0;i<hist.length;i+=2){const li=document.createElement("li");li.textContent=(Math.floor(i/2)+1)+". "+hist[i].san+(hist[i+1]?"  "+hist[i+1].san:"");$("history").append(li);}
}
function tap(sq){
  if(game.isGameOver() || (mode==="online" && (!socket || socket.readyState!==1 || waiting || game.turn()!==myColor))) return;
  const piece=game.get(sq);
  if(selected && legal.some(m=>m.to===sq)){
    const move=game.moves({square:selected,verbose:true}).find(m=>m.to===sq);
    if(!move)return;
    let promotion=move.flags.includes("p")?"q":undefined;
    const uci=selected+sq+(promotion||"");
    if(mode==="online"){socket.send(JSON.stringify({type:"move",uci}));selected=null;legal=[];return;}
    game.move({from:selected,to:sq,promotion});selected=null;legal=[];render();return;
  }
  if(piece && (mode!=="online" || piece.color===myColor)){
    selected=sq;legal=game.moves({square:sq,verbose:true});render();return;
  }
  selected=null;legal=[];render();
}
function setNotice(s){$("room-status").textContent=s;}
function closeSocket(){if(socket){socket.close();socket=null;}myColor=null;room=null;waiting=false;$("room-code-box").hidden=true;$("connection").textContent="Offline";}
function connect(action){
  const url=$("server").value.trim();
  if(!/^wss?:\/\//.test(url)){setNotice("Enter a valid ws:// or wss:// server address.");return;}
  closeSocket();setNotice("Connecting…");$("connection").textContent="Connecting";
  try{socket=new WebSocket(url);}catch(e){setNotice("Could not open WebSocket: "+e.message);return;}
  socket.onopen=()=>{socket.send(JSON.stringify(action));$("connection").textContent="Connected";};
  socket.onmessage=event=>{
    let msg;try{msg=JSON.parse(event.data);}catch{return;}
    if(msg.type==="created"||msg.type==="joined"){
      room=msg.code;myColor=msg.color;game.load(msg.fen);selected=null;legal=[];waiting=false;
      $("room-code").textContent=room;$("room-code-box").hidden=false;setNotice("Room "+room+" · You are "+myColor+".");render();
    }else if(msg.type==="opponent_joined"){setNotice("Opponent joined. White moves first.");waiting=false;}
    else if(msg.type==="move"){
      if(msg.fen)game.load(msg.fen);
      selected=null;legal=[];render();
    }else if(msg.type==="error"){setNotice(msg.message||"Server error.");}
    else if(msg.type==="opponent_disconnected"){setNotice("Opponent disconnected.");}
    else if(msg.type==="game_over"){setNotice("Game over: "+(msg.result||"finished"));}
    else if(msg.type==="draw_offer"){setNotice("Your opponent offered a draw.");}
  };
  socket.onerror=()=>setNotice("Connection failed. Check the server address and try again.");
  socket.onclose=()=>{$("connection").textContent="Offline";if(mode==="online")setNotice("Disconnected from server.");};
}
$("create-room").addEventListener("click",()=>{mode="online";connect({type:"create"});});
$("join-room").addEventListener("click",()=>{const code=$("room-code-input").value.trim().toUpperCase();if(code.length!==6){setNotice("Enter the 6-character room code.");return;}mode="online";connect({type:"join",code});});
$("copy-code").addEventListener("click",async()=>{try{await navigator.clipboard.writeText(room);setNotice("Room code copied.");}catch{setNotice("Copy unavailable. Select the code to copy it.");}});
document.querySelectorAll(".tab").forEach(btn=>btn.addEventListener("click",()=>{
  const next=btn.dataset.mode;if(mode==="online"&&next==="local")closeSocket();
  mode=next;document.querySelectorAll(".tab").forEach(b=>b.classList.toggle("active",b===btn));
  $("local-panel").hidden=next!=="local";$("online-panel").hidden=next!=="online";
  selected=null;legal=[];render();
}));
$("new-game").addEventListener("click",()=>{if(mode==="online"){setNotice("Start a new online room to begin another game.");return;}game.reset();selected=null;legal=[];render();});
$("undo").addEventListener("click",()=>{if(mode==="online"){setNotice("Undo is disabled in online games.");return;}game.undo();selected=null;legal=[];render();});
$("flip").addEventListener("click",()=>{flipped=!flipped;render();});
render();
