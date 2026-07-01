//! Client-facing landing page served at `/`. A polished, bilingual (ES/EN)
//! showcase whose hero is a **live** chat wired to `POST /api/chat` — the same
//! agent brain as WhatsApp (real Claude, scoring, Google Calendar, HubSpot).
//! Branding (`__AGENT__`, `__BUSINESS__`, `__SUMMARY__`) is injected live from
//! the loaded `offer.json`.

use axum::extract::State;
use axum::response::Html;

use crate::state::AppState;

pub async fn landing(State(state): State<AppState>) -> Html<String> {
    let b = &state.offer.branding;
    let o = &state.offer.offer;
    let page = TEMPLATE
        .replace("__AGENT__", &esc(&b.agent_name))
        .replace("__BUSINESS__", &esc(&b.business_name))
        .replace("__SUMMARY__", &esc(&o.summary))
        .replace("__MODEL__", &esc(&state.config.anthropic.model));
    Html(page)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const TEMPLATE: &str = r##"<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="description" content="__SUMMARY__">
<meta name="generator" content="Claude __MODEL__">
<title>__AGENT__ · __BUSINESS__</title>
<style>
  :root{
    --bg:#05070e; --bg2:#0a0f1d; --card:#0e1526; --card2:#111a30; --line:#1e2c48;
    --txt:#eaf1fb; --muted:#93a7c9; --dim:#63769a;
    --green:#25d366; --green2:#12b855; --emerald:#34d399; --accent:#6ea8fe; --hot:#ff5d5d;
    --glow:0 0 40px rgba(37,211,102,.35);
  }
  *{box-sizing:border-box}
  html{scroll-behavior:smooth}
  body{
    margin:0; color:var(--txt);
    font:16px/1.6 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,system-ui,sans-serif;
    background:
      radial-gradient(1200px 700px at 8% -8%, #10321f 0%, transparent 55%),
      radial-gradient(1000px 700px at 100% 0%, #10203f 0%, transparent 52%),
      radial-gradient(900px 900px at 50% 120%, #0c2a1c 0%, transparent 60%),
      linear-gradient(180deg,var(--bg),var(--bg2));
    -webkit-font-smoothing:antialiased; overflow-x:hidden;
  }
  a{color:inherit; text-decoration:none}
  .wrap{max-width:1120px; margin:0 auto; padding:0 24px}
  #rain{position:fixed; inset:0; z-index:0; opacity:.12; pointer-events:none}
  .layer{position:relative; z-index:1}
  nav{position:sticky; top:0; z-index:50; backdrop-filter:blur(12px);
    background:linear-gradient(180deg,rgba(5,7,14,.85),rgba(5,7,14,.4)); border-bottom:1px solid rgba(30,44,72,.6)}
  .nav{display:flex; align-items:center; gap:18px; height:64px}
  .brand{display:flex; align-items:center; gap:10px; font-weight:800; letter-spacing:-.2px}
  .logo{width:30px; height:30px; border-radius:9px; background:linear-gradient(135deg,var(--green),var(--emerald));
    display:grid; place-items:center; color:#04120a; font-weight:900; box-shadow:var(--glow)}
  .navlinks{display:flex; gap:22px; margin-left:14px}
  .navlinks a{color:var(--muted); font-size:14px; font-weight:600; transition:.15s}
  .navlinks a:hover{color:var(--txt)}
  .navright{margin-left:auto; display:flex; align-items:center; gap:12px}
  .langtoggle{display:inline-flex; border:1px solid var(--line); border-radius:999px; overflow:hidden}
  .lang-btn{background:transparent; color:var(--dim); border:0; padding:6px 12px; font-size:11.5px; font-weight:800; cursor:pointer; letter-spacing:.5px; transition:.15s}
  .lang-btn.active{background:var(--accent); color:#04101f}
  .btn{display:inline-flex; align-items:center; gap:8px; font-weight:700; font-size:14px; border-radius:11px; padding:11px 18px; cursor:pointer; border:1px solid transparent; transition:.18s; white-space:nowrap}
  .btn.wa{background:linear-gradient(135deg,var(--green),var(--green2)); color:#04120a; box-shadow:0 10px 30px rgba(37,211,102,.35)}
  .btn.wa:hover{transform:translateY(-2px); box-shadow:0 16px 40px rgba(37,211,102,.5)}
  .btn.ghost{background:rgba(255,255,255,.04); border-color:var(--line); color:var(--txt)}
  .btn.ghost:hover{border-color:var(--accent); color:#fff}
  .btn.sm{padding:9px 15px; font-size:13px}
  .hero{display:flex; flex-direction:column; align-items:center; text-align:center; gap:12px; padding:52px 0 44px}
  .herotop{max-width:800px; margin:0 auto}
  .eyebrow{display:inline-flex; align-items:center; gap:8px; font-size:12.5px; font-weight:700; color:var(--emerald);
    background:rgba(37,211,102,.08); border:1px solid rgba(37,211,102,.28); padding:6px 13px; border-radius:999px}
  .pdot{width:8px;height:8px;border-radius:50%;background:var(--green);box-shadow:0 0 0 0 rgba(37,211,102,.6);animation:pulse 1.8s infinite}
  @keyframes pulse{0%{box-shadow:0 0 0 0 rgba(37,211,102,.5)}70%{box-shadow:0 0 0 10px rgba(37,211,102,0)}100%{box-shadow:0 0 0 0 rgba(37,211,102,0)}}
  h1{font-size:clamp(30px,4.4vw,52px); line-height:1.06; letter-spacing:-1.2px; margin:18px 0 16px; font-weight:850}
  h1 .g{background:linear-gradient(120deg,var(--emerald),var(--green) 55%,var(--accent)); -webkit-background-clip:text; background-clip:text; color:transparent}
  .lead{font-size:17.5px; color:var(--muted); max-width:40em; margin:0 auto 16px}
  .cta{display:flex; gap:12px; flex-wrap:wrap}
  .trust{display:flex; gap:18px; flex-wrap:wrap; justify-content:center; margin-top:14px; color:var(--dim); font-size:13px}
  .trust span{display:inline-flex; align-items:center; gap:7px}
  .trust b{color:var(--txt); font-weight:700}
  .tick{color:var(--emerald)}
  .stage{position:relative; display:flex; justify-content:center}
  .livetag{position:absolute; top:-14px; left:50%; transform:translateX(-50%); z-index:4; font-size:11px; font-weight:800; letter-spacing:.4px;
    color:#04120a; background:linear-gradient(135deg,var(--green),var(--emerald)); padding:5px 12px; border-radius:999px; box-shadow:var(--glow); display:inline-flex; align-items:center; gap:6px}
  .livetag .pdot{background:#04120a}
  .phone{position:relative; width:320px; height:640px; border-radius:42px; padding:13px;
    background:linear-gradient(160deg,#1b2540,#0a0f1d); border:1px solid #26375c;
    box-shadow:0 40px 90px rgba(0,0,0,.6), inset 0 1px 0 rgba(255,255,255,.06); z-index:2}
  .screen{height:100%; border-radius:31px; overflow:hidden; background:#0b141a; display:flex; flex-direction:column}
  .wahead{background:linear-gradient(180deg,#128c3e,#075e2f); padding:14px 15px 12px; display:flex; align-items:center; gap:11px; color:#fff}
  .wapic{width:38px;height:38px;border-radius:50%;background:linear-gradient(135deg,var(--emerald),var(--accent)); display:grid; place-items:center; font-weight:800; color:#04120a; font-size:16px}
  .waname{font-weight:700; font-size:14.5px; line-height:1.2}
  .wastat{font-size:11px; color:#bff0d0; display:flex; align-items:center; gap:5px}
  .chat{flex:1; padding:14px 12px; overflow-y:auto; display:flex; flex-direction:column; gap:9px;
    background:linear-gradient(180deg,#0b141a,#0d171e);
    background-image:radial-gradient(rgba(255,255,255,.02) 1px, transparent 1px); background-size:16px 16px}
  .chat::-webkit-scrollbar{width:5px} .chat::-webkit-scrollbar-thumb{background:#22323c; border-radius:9px}
  .bub{max-width:82%; padding:8px 11px; border-radius:13px; font-size:13.2px; line-height:1.42; opacity:0; transform:translateY(8px); animation:rise .3s forwards; box-shadow:0 1px 2px rgba(0,0,0,.3); position:relative; white-space:pre-wrap; word-wrap:break-word}
  @keyframes rise{to{opacity:1; transform:none}}
  .bub.in{align-self:flex-start; background:#1f2c33; color:#e9f3ee; border-top-left-radius:4px}
  .bub.out{align-self:flex-end; background:#075e54; color:#eafff2; border-top-right-radius:4px}
  .bub .tm{display:block; font-size:9.5px; color:rgba(255,255,255,.45); text-align:right; margin-top:3px}
  .typing{align-self:flex-start; background:#1f2c33; padding:11px 13px; border-radius:13px; border-top-left-radius:4px; display:inline-flex; gap:4px; opacity:0; animation:rise .3s forwards}
  .typing i{width:6px;height:6px;border-radius:50%;background:#7d93a8; animation:blink 1.2s infinite}
  .typing i:nth-child(2){animation-delay:.2s} .typing i:nth-child(3){animation-delay:.4s}
  @keyframes blink{0%,60%,100%{opacity:.3; transform:translateY(0)}30%{opacity:1; transform:translateY(-3px)}}
  .wainput{display:flex; gap:8px; padding:10px; background:#0b141a; border-top:1px solid #12202a}
  .wainput input{flex:1; background:#1f2c33; border:1px solid #2a3b44; border-radius:20px; color:var(--txt); padding:9px 14px; font-size:13px; outline:none}
  .wainput input:focus{border-color:var(--emerald)}
  .wainput button{width:38px;height:38px;flex:0 0 38px;border-radius:50%; border:0; background:linear-gradient(135deg,var(--green),var(--green2)); color:#04120a; font-size:15px; cursor:pointer; display:grid;place-items:center; transition:.15s}
  .wainput button:hover{transform:scale(1.06)} .wainput button:disabled{opacity:.45; cursor:default; transform:none}
  .float{position:absolute; z-index:3; border-radius:14px; padding:11px 13px; font-size:12.5px; font-weight:700;
    background:linear-gradient(160deg,var(--card2),var(--card)); border:1px solid var(--line); box-shadow:0 18px 44px rgba(0,0,0,.5); opacity:0; pointer-events:none}
  .float.show{animation:pop .5s forwards}
  @keyframes pop{0%{opacity:0; transform:translateY(10px) scale(.9)}100%{opacity:1; transform:none}}
  .float.hot{top:88px; right:-18px; color:var(--hot); border-color:rgba(255,93,93,.4)}
  .float.hot .bar{margin-top:6px; height:5px; border-radius:3px; background:#2a1620; overflow:hidden; width:120px}
  .float.hot .bar i{display:block; height:100%; width:0; background:linear-gradient(90deg,#ff8a5d,var(--hot)); animation:fill 1.1s .1s forwards}
  @keyframes fill{to{width:88%}}
  .float.book{bottom:96px; left:-26px; color:var(--emerald); border-color:rgba(52,211,153,.4)}
  .float small{display:block; color:var(--dim); font-weight:600; font-size:10.5px; margin-top:2px}
  section{position:relative; padding:64px 0}
  .kicker{font-size:12.5px; font-weight:800; letter-spacing:.18em; text-transform:uppercase; color:var(--emerald); text-align:center}
  h2{font-size:clamp(24px,3vw,36px); text-align:center; margin:10px 0 8px; letter-spacing:-.6px; font-weight:820}
  .sectlead{text-align:center; color:var(--muted); max-width:34em; margin:0 auto 40px}
  .steps{display:grid; grid-template-columns:repeat(4,1fr); gap:16px}
  .capgrid{display:grid; grid-template-columns:repeat(auto-fit,minmax(250px,1fr)); gap:16px}
  .stepc{background:linear-gradient(180deg,rgba(255,255,255,.03),transparent); border:1px solid var(--line); border-radius:18px; padding:24px 18px; position:relative; overflow:hidden}
  .stepc::before{content:""; position:absolute; inset:0; background:radial-gradient(300px 120px at 50% -20%, rgba(37,211,102,.16), transparent 70%); opacity:0; transition:.4s}
  .stepc:hover::before{opacity:1}
  .stepc .n{font-size:12px; font-weight:800; color:var(--dim); letter-spacing:.1em}
  .stepc .ic{font-size:30px; margin:8px 0 10px; display:block}
  .stepc h3{margin:0 0 6px; font-size:17px}
  .stepc p{margin:0; color:var(--muted); font-size:13.5px}
  .reveal{opacity:0; transform:translateY(22px); transition:.7s cubic-bezier(.2,.7,.2,1)}
  .reveal.in{opacity:1; transform:none}
  .caps{display:grid; grid-template-columns:repeat(4,1fr); gap:16px}
  .cap{text-align:center; background:linear-gradient(180deg,rgba(255,255,255,.03),transparent); border:1px solid var(--line); border-radius:16px; padding:26px 14px}
  .cap .v{font-size:30px; font-weight:850; background:linear-gradient(120deg,var(--emerald),var(--accent)); -webkit-background-clip:text; background-clip:text; color:transparent; letter-spacing:-.5px}
  .cap .k{color:var(--muted); font-size:13px; margin-top:4px}
  .integ{display:flex; gap:14px; justify-content:center; flex-wrap:wrap; margin-top:8px}
  .chip{display:inline-flex; align-items:center; gap:9px; border:1px solid var(--line); border-radius:12px; padding:12px 18px; background:rgba(255,255,255,.02); font-weight:700; font-size:14px}
  .chip .cd{width:9px;height:9px;border-radius:50%}
  .ctaband{background:linear-gradient(160deg,rgba(37,211,102,.10),rgba(110,168,254,.06)); border:1px solid var(--line); border-radius:24px; padding:46px 30px; text-align:center; position:relative; overflow:hidden}
  .ctaband::after{content:""; position:absolute; width:400px;height:400px; right:-120px; top:-160px; background:radial-gradient(circle,rgba(37,211,102,.22),transparent 65%)}
  .ctaband h2{margin-bottom:10px}
  .ctaband .cta{justify-content:center; margin-top:22px}
  footer{border-top:1px solid var(--line); padding:30px 0; color:var(--dim); font-size:13px; display:flex; justify-content:space-between; flex-wrap:wrap; gap:10px}
  footer b{color:var(--muted)}
  @media(max-width:860px){
    .hero{grid-template-columns:1fr; padding-top:44px} .stage{margin-top:26px}
    .navlinks{display:none}
    .steps,.caps{grid-template-columns:repeat(2,1fr)}
  }
  @media(max-width:520px){ .caps{grid-template-columns:1fr 1fr} .float.hot{right:-6px} .float.book{left:-6px} }
</style>
</head>
<body>
<canvas id="rain"></canvas>
<div class="layer">
<nav><div class="wrap nav">
  <div class="brand"><span class="logo">◆</span> <span>__AGENT__</span></div>
  <div class="navlinks">
    <a href="#como" data-es="Cómo funciona" data-en="How it works">Cómo funciona</a>
    <a href="#demo" data-es="Demo en vivo" data-en="Live demo">Demo en vivo</a>
    <a href="#contacto" data-es="Contacto" data-en="Contact">Contacto</a>
  </div>
  <div class="navright">
    <span class="langtoggle" role="group" aria-label="Language">
      <button type="button" class="lang-btn" data-lang="es">ES</button>
      <button type="button" class="lang-btn" data-lang="en">EN</button>
    </span>
    <a class="btn wa sm" href="https://creandotumatrix.com/#servicios" target="_blank" rel="noopener">
      <span data-es="Agendar llamada" data-en="Book a Call">Agendar llamada</span>
    </a>
  </div>
</div></nav>

<header class="wrap hero" id="demo">
  <div class="herotop">
    <span class="eyebrow"><span class="pdot"></span> <span data-es="Agente en vivo · pruébalo ahora" data-en="Live agent · try it now">Agente en vivo · pruébalo ahora</span></span>
    <h1><span data-es="Convierte cada WhatsApp en una" data-en="Turn every WhatsApp into a">Convierte cada WhatsApp en una</span> <span class="g" data-es="cita agendada" data-en="booked meeting">cita agendada</span>.</h1>
    <p class="lead" data-es="__AGENT__ atiende, califica y agenda a tus prospectos automáticamente — 24/7, en español e inglés. Escríbele aquí abajo: es el agente real, en vivo." data-en="__AGENT__ greets, qualifies and books your prospects automatically — 24/7, in Spanish and English. Chat with it below: it's the real agent, live.">__AGENT__ atiende, califica y agenda a tus prospectos automáticamente — 24/7, en español e inglés. Escríbele aquí abajo: es el agente real, en vivo.</p>
    <div class="trust">
      <span><span class="tick">✓</span> <b>24/7</b> <span data-es="sin descanso" data-en="always on">sin descanso</span></span>
      <span><span class="tick">✓</span> <b>ES / EN</b></span>
      <span><span class="tick">✓</span> <span data-es="Agenda + CRM reales" data-en="Real calendar + CRM">Agenda + CRM reales</span></span>
    </div>
  </div>
  <div class="stage">
    <span class="livetag"><span class="pdot"></span> <span data-es="EN VIVO" data-en="LIVE">EN VIVO</span></span>
    <div class="phone">
      <div class="screen">
        <div class="wahead">
          <div class="wapic">✦</div>
          <div>
            <div class="waname">__AGENT__</div>
            <div class="wastat"><span style="width:6px;height:6px;border-radius:50%;background:#8affc0"></span> <span data-es="en línea" data-en="online">en línea</span></div>
          </div>
        </div>
        <div class="chat" id="chat"></div>
        <div class="wainput">
          <input id="cin" type="text" autocomplete="off" placeholder="Escribe tu mensaje…" data-esph="Escribe tu mensaje…" data-enph="Type your message…">
          <button id="csend" type="button" aria-label="Send">➤</button>
        </div>
      </div>
    </div>
    <div class="float hot" id="fhot">🔥 <span data-es="Lead CALIENTE" data-en="HOT lead">Lead CALIENTE</span><div class="bar"><i></i></div><small data-es="Listo para agendar" data-en="Ready to book">Listo para agendar</small></div>
    <div class="float book" id="fbook">✓ <span data-es="Cita agendada" data-en="Meeting booked">Cita agendada</span><small data-es="Google Calendar" data-en="Google Calendar">Google Calendar</small></div>
  </div>
</header>

<section id="como">
  <div class="wrap">
    <div class="kicker" data-es="Cómo funciona" data-en="How it works">Cómo funciona</div>
    <h2 data-es="De “hola” a cita agendada, solo" data-en="From “hi” to booked, on its own">De “hola” a cita agendada, solo</h2>
    <p class="sectlead" data-es="Cuatro pasos que tu asistente hace en cada conversación, sin que tú levantes un dedo." data-en="Four steps your assistant runs in every conversation — hands-free.">Cuatro pasos que tu asistente hace en cada conversación, sin que tú levantes un dedo.</p>
    <div class="steps">
      <div class="stepc reveal"><span class="n">01</span><span class="ic">💬</span><h3 data-es="Atiende" data-en="Engage">Atiende</h3><p data-es="Responde al instante a cada mensaje, día y noche, en su idioma." data-en="Replies instantly to every message, day and night, in their language.">Responde al instante a cada mensaje, día y noche, en su idioma.</p></div>
      <div class="stepc reveal"><span class="n">02</span><span class="ic">🎯</span><h3 data-es="Califica" data-en="Qualify">Califica</h3><p data-es="Hace las preguntas correctas y puntúa cada lead: caliente, tibio o frío." data-en="Asks the right questions and scores each lead: hot, warm or cold.">Hace las preguntas correctas y puntúa cada lead: caliente, tibio o frío.</p></div>
      <div class="stepc reveal"><span class="n">03</span><span class="ic">📅</span><h3 data-es="Agenda" data-en="Book">Agenda</h3><p data-es="Ofrece horarios reales de tu calendario y reserva la cita al momento." data-en="Offers real slots from your calendar and books the meeting on the spot.">Ofrece horarios reales de tu calendario y reserva la cita al momento.</p></div>
      <div class="stepc reveal"><span class="n">04</span><span class="ic">📨</span><h3 data-es="Entrega" data-en="Hand off">Entrega</h3><p data-es="Registra el lead en tu CRM y avisa a tu asesor con todo el contexto." data-en="Logs the lead in your CRM and alerts your rep with full context.">Registra el lead en tu CRM y avisa a tu asesor con todo el contexto.</p></div>
    </div>
  </div>
</section>

<section id="capacidades">
  <div class="wrap">
    <div class="kicker" data-es="Todo lo que hace" data-en="Everything it does">Todo lo que hace</div>
    <h2 data-es="Una conversación, todo el embudo" data-en="One conversation, the whole funnel">Una conversación, todo el embudo</h2>
    <p class="sectlead" data-es="Agendar la cita es solo un paso. Esto es todo lo que el agente hace, solo, en cada chat." data-en="Booking is just one step — here's everything the agent does, on its own, in every chat.">Agendar la cita es solo un paso. Esto es todo lo que el agente hace, solo, en cada chat.</p>
    <div class="capgrid">
      <div class="stepc reveal"><span class="ic">⚡</span><h3 data-es="Respuesta 24/7" data-en="24/7 replies">Respuesta 24/7</h3><p data-es="Ningún lead espera; ninguno se pierde por responder tarde." data-en="No lead waits; none lost to a slow reply.">Ningún lead espera; ninguno se pierde por responder tarde.</p></div>
      <div class="stepc reveal"><span class="ic">💬</span><h3 data-es="Conversa, no da menús" data-en="Talks, not menus">Conversa, no da menús</h3><p data-es="Razona en vivo con Claude, una pregunta a la vez, en español o inglés." data-en="Reasons live with Claude, one question at a time, in Spanish or English.">Razona en vivo con Claude, una pregunta a la vez, en español o inglés.</p></div>
      <div class="stepc reveal"><span class="ic">🎯</span><h3 data-es="Califica y puntúa" data-en="Qualifies and scores">Califica y puntúa</h3><p data-es="Extrae necesidad, urgencia y tamaño; puntúa hot / warm / cold con una rúbrica determinista." data-en="Extracts need, urgency and size; scores hot / warm / cold on a deterministic rubric.">Extrae necesidad, urgencia y tamaño; puntúa hot / warm / cold con una rúbrica determinista.</p></div>
      <div class="stepc reveal"><span class="ic">🛡️</span><h3 data-es="Filtra lo que no aplica" data-en="Filters out bad-fit">Filtra lo que no aplica</h3><p data-es="Detecta consultas fuera de lugar (tareas, empleo) y se despide con amabilidad." data-en="Spots off-topic queries (homework, job seekers) and bows out kindly.">Detecta consultas fuera de lugar (tareas, empleo) y se despide con amabilidad.</p></div>
      <div class="stepc reveal"><span class="ic">📅</span><h3 data-es="Lee tu agenda y reserva" data-en="Reads your calendar and books">Lee tu agenda y reserva</h3><p data-es="Ofrece solo horarios realmente libres y crea la cita real en tu Google Calendar." data-en="Offers only genuinely open slots and creates the real event on your Google Calendar.">Ofrece solo horarios realmente libres y crea la cita real en tu Google Calendar.</p></div>
      <div class="stepc reveal"><span class="ic">🗂️</span><h3 data-es="Registra el trato en tu CRM" data-en="Logs the deal in your CRM">Registra el trato en tu CRM</h3><p data-es="Crea contacto y trato en HubSpot con puntaje, razones y datos calificados." data-en="Creates a HubSpot contact and deal with score, reasons and qualified fields.">Crea contacto y trato en HubSpot con puntaje, razones y datos calificados.</p></div>
      <div class="stepc reveal"><span class="ic">📨</span><h3 data-es="Entrega al asesor" data-en="Hands off to your rep">Entrega al asesor</h3><p data-es="Avisa a tu equipo con resumen, puntaje, contacto y enlace a la transcripción." data-en="Notifies your team with summary, score, contact and a transcript link.">Avisa a tu equipo con resumen, puntaje, contacto y enlace a la transcripción.</p></div>
      <div class="stepc reveal"><span class="ic">🧊</span><h3 data-es="Recupera leads fríos" data-en="Recovers cold leads">Recupera leads fríos</h3><p data-es="En vez de agendar, comparte un recurso útil y protege el tiempo de tu asesor." data-en="Instead of booking, shares a helpful resource and protects your rep's time.">En vez de agendar, comparte un recurso útil y protege el tiempo de tu asesor.</p></div>
      <div class="stepc reveal"><span class="ic">🧠</span><h3 data-es="Recuerda todo, sin improvisar precios" data-en="Remembers all, never guesses price">Recuerda todo, sin improvisar precios</h3><p data-es="Guarda cada mensaje, lead y cita; responde dudas y deja el precio para la llamada." data-en="Keeps every message, lead and booking; answers FAQs and leaves pricing to the call.">Guarda cada mensaje, lead y cita; responde dudas y deja el precio para la llamada.</p></div>
    </div>
  </div>
</section>

<section>
  <div class="wrap">
    <div class="caps">
      <div class="cap reveal"><div class="v">24/7</div><div class="k" data-es="Atención sin horarios" data-en="No business hours">Atención sin horarios</div></div>
      <div class="cap reveal"><div class="v">&lt;30s</div><div class="k" data-es="Primera respuesta" data-en="First reply">Primera respuesta</div></div>
      <div class="cap reveal"><div class="v">2</div><div class="k" data-es="Idiomas (ES/EN)" data-en="Languages (ES/EN)">Idiomas (ES/EN)</div></div>
      <div class="cap reveal"><div class="v">∞</div><div class="k" data-es="Chats a la vez" data-en="Chats at once">Chats a la vez</div></div>
    </div>
    <div class="integ" style="margin-top:30px">
      <span class="chip"><span class="cd" style="background:var(--green)"></span> WhatsApp</span>
      <span class="chip"><span class="cd" style="background:var(--accent)"></span> Google Calendar</span>
      <span class="chip"><span class="cd" style="background:#ff7a59"></span> HubSpot CRM</span>
      <span class="chip"><span class="cd" style="background:#5b9bd5"></span> Postgres Database</span>
    </div>
  </div>
</section>

<section id="contacto">
  <div class="wrap">
    <div class="ctaband">
      <div class="kicker" data-es="Demo" data-en="Demo">Demo</div>
      <h2 data-es="Ya lo probaste arriba — ahora llévalo a tu negocio" data-en="You just tried it above — now bring it to your business">Ya lo probaste arriba — ahora llévalo a tu negocio</h2>
      <p class="sectlead" data-es="El chat de arriba es el agente real trabajando en vivo. Hablemos de conectarlo a tu WhatsApp, tu calendario y tu CRM." data-en="The chat above is the real agent working live. Let's talk about wiring it to your WhatsApp, calendar and CRM.">El chat de arriba es el agente real trabajando en vivo. Hablemos de conectarlo a tu WhatsApp, tu calendario y tu CRM.</p>
      <div class="cta">
        <a class="btn wa" href="https://wa.me/523223500097?text=Hola,%20quiero%20la%20demo%20de%20__AGENT__" target="_blank" rel="noopener"><span>▶</span> <span data-es="Escribir por WhatsApp" data-en="Message on WhatsApp">Escribir por WhatsApp</span></a>
        <a class="btn ghost" href="mailto:marcus@creandotumatrix.com?subject=Demo%20__AGENT__">marcus@creandotumatrix.com</a>
      </div>
    </div>
  </div>
</section>

<footer class="wrap">
  <span><b>__BUSINESS__</b> · __AGENT__</span>
  <span data-es="Asistente comercial de WhatsApp" data-en="WhatsApp sales assistant">Asistente comercial de WhatsApp</span>
</footer>
</div>

<script>
(function(){
  var c=document.getElementById('rain'), x=c.getContext('2d'), cols, drops, W, H;
  function size(){ W=c.width=innerWidth; H=c.height=innerHeight; cols=Math.floor(W/16); drops=Array(cols).fill(0).map(function(){return Math.random()*-40;}); }
  size(); addEventListener('resize', size);
  var chars='01<>{}/#$LEADGEN01'.split('');
  function draw(){
    x.fillStyle='rgba(5,7,14,.09)'; x.fillRect(0,0,W,H);
    x.fillStyle='#25d366'; x.font='13px monospace';
    for(var i=0;i<cols;i++){
      var t=chars[Math.floor(Math.random()*chars.length)];
      x.fillText(t, i*16, drops[i]*16);
      if(drops[i]*16>H && Math.random()>.975) drops[i]=0;
      drops[i]++;
    }
    requestAnimationFrame(draw);
  }
  draw();
})();

(function(){
  var io=new IntersectionObserver(function(es){ es.forEach(function(e){ if(e.isIntersecting){ e.target.classList.add('in'); io.unobserve(e.target); } }); },{threshold:.18});
  document.querySelectorAll('.reveal').forEach(function(el,i){ el.style.transitionDelay=(i%4*80)+'ms'; io.observe(el); });
})();

/* LIVE chat: talks to the real agent via POST /api/chat (no WhatsApp). */
(function(){
  var chat=document.getElementById('chat');
  var input=document.getElementById('cin');
  var sendBtn=document.getElementById('csend');
  var fhot=document.getElementById('fhot'), fbook=document.getElementById('fbook');
  var sid='web-'+Date.now().toString(36)+Math.random().toString(36).slice(2,9);
  var busy=false;
  function now(){ var d=new Date(); return (d.getHours()%12||12)+':'+String(d.getMinutes()).padStart(2,'0'); }
  function esc(s){ return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
  function bubble(text,out){
    var b=document.createElement('div'); b.className='bub '+(out?'out':'in');
    b.innerHTML=esc(text)+'<span class="tm">'+now()+(out?' ✓✓':'')+'</span>';
    chat.appendChild(b); chat.scrollTop=chat.scrollHeight;
  }
  function showTyping(){ var t=document.createElement('div'); t.className='typing'; t.id='typing'; t.innerHTML='<i></i><i></i><i></i>'; chat.appendChild(t); chat.scrollTop=chat.scrollHeight; }
  function hideTyping(){ var t=document.getElementById('typing'); if(t) t.remove(); }
  function cards(score,status){
    if(score==='hot' || score==='warm') fhot.classList.add('show');
    if(status==='booked') fbook.classList.add('show');
  }
  async function send(text,showUser){
    if(busy) return; busy=true; sendBtn.disabled=true;
    if(showUser) bubble(text,true);
    showTyping();
    try{
      var r=await fetch('/api/chat',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({session_id:sid,text:text})});
      var d=await r.json();
      hideTyping();
      if(d.reply) bubble(d.reply,false);
      cards(d.score,d.status);
    }catch(e){
      hideTyping(); bubble('Se perdió la conexión. Intenta de nuevo. 🙏',false);
    }
    busy=false; sendBtn.disabled=false; input.focus();
  }
  function submit(){ var t=input.value.trim(); if(!t||busy) return; input.value=''; send(t,true); }
  sendBtn.addEventListener('click',submit);
  input.addEventListener('keydown',function(e){ if(e.key==='Enter'){ e.preventDefault(); submit(); } });
  var openLang=(navigator.language||'es').toLowerCase().indexOf('en')===0?'en':'es';
  send(openLang==='en'?'Hi':'Hola', true);
})();

/* Language toggle: swaps static UI copy (chat content is whatever the agent says). */
(function(){
  var KEY='lg_lang';
  var els=document.querySelectorAll('[data-es]');
  var input=document.getElementById('cin');
  function apply(l){
    if(l!=='en') l='es';
    document.documentElement.lang=l;
    els.forEach(function(el){ var v=el.getAttribute('data-'+l); if(v!==null) el.textContent=v; });
    if(input){ var ph=input.getAttribute('data-'+l+'ph'); if(ph) input.placeholder=ph; }
    document.querySelectorAll('.lang-btn').forEach(function(b){ b.classList.toggle('active', b.getAttribute('data-lang')===l); });
    try{ localStorage.setItem(KEY,l); }catch(e){}
  }
  document.querySelectorAll('.lang-btn').forEach(function(b){ b.addEventListener('click', function(e){ apply(e.currentTarget.getAttribute('data-lang')); }); });
  var saved=null; try{ saved=localStorage.getItem(KEY); }catch(e){}
  var def=(navigator.language||'es').toLowerCase().indexOf('en')===0?'en':'es';
  apply(saved||def);
})();
</script>
</body>
</html>"##;
