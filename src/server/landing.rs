//! Polished bilingual (ES/EN) landing / status page served at `/`. Gives the
//! deployed service a presentable face for demos and doubles as an HTTP
//! healthcheck target. Branding is pulled live from the loaded `offer.json`;
//! the UI chrome toggles language client-side (browser-default, persisted).

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

const TEMPLATE: &str = r#"<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>__AGENT__ · Asistente Comercial</title>
<style>
  :root{
    --bg:#0b1020; --bg2:#11182f; --card:#0f1730; --line:#22304f;
    --txt:#e8edf7; --muted:#8aa0c6; --brand:#22c55e; --accent:#6ea8fe; --wa:#25d366;
  }
  *{box-sizing:border-box}
  html,body{margin:0;height:100%}
  body{
    font:16px/1.55 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,system-ui,sans-serif;
    color:var(--txt);
    background:
      radial-gradient(1100px 600px at 12% -10%, #1b2748 0%, transparent 55%),
      radial-gradient(900px 600px at 100% 0%, #14233f 0%, transparent 50%),
      linear-gradient(180deg,var(--bg),var(--bg2));
    min-height:100%;
    display:flex; align-items:center; justify-content:center; padding:32px;
  }
  .card{
    width:100%; max-width:760px; background:linear-gradient(180deg,rgba(255,255,255,.03),rgba(255,255,255,0));
    border:1px solid var(--line); border-radius:20px; padding:34px 36px;
    box-shadow:0 24px 70px rgba(0,0,0,.45);
  }
  .row{display:flex; align-items:center; gap:12px; flex-wrap:wrap}
  .badge{
    display:inline-flex; align-items:center; gap:7px; font-size:12.5px; font-weight:600;
    color:var(--brand); background:rgba(34,197,94,.10); border:1px solid rgba(34,197,94,.30);
    padding:5px 11px; border-radius:999px; letter-spacing:.2px;
  }
  .dot{width:8px;height:8px;border-radius:50%;background:var(--brand);box-shadow:0 0 0 0 rgba(34,197,94,.6);animation:p 1.8s infinite}
  @keyframes p{0%{box-shadow:0 0 0 0 rgba(34,197,94,.55)}70%{box-shadow:0 0 0 9px rgba(34,197,94,0)}100%{box-shadow:0 0 0 0 rgba(34,197,94,0)}}
  .langtoggle{margin-left:auto; display:inline-flex; border:1px solid var(--line); border-radius:999px; overflow:hidden}
  .lang-btn{background:transparent; color:var(--muted); border:0; padding:5px 13px; font-size:12px; font-weight:700; cursor:pointer; letter-spacing:.4px; transition:.15s}
  .lang-btn.active{background:var(--accent); color:#091022}
  .lang-btn:not(.active):hover{color:var(--txt)}
  h1{font-size:30px; margin:18px 0 4px; letter-spacing:-.3px}
  h1 span{color:var(--accent)}
  .sub{color:var(--muted); margin:0 0 22px; font-size:15px}
  .summary{font-size:16px; color:#cdd9f0; margin:0 0 26px}
  .pipe{display:grid; grid-template-columns:repeat(4,1fr); gap:10px; margin:0 0 26px}
  .step{background:var(--card); border:1px solid var(--line); border-radius:14px; padding:14px 12px; text-align:center; position:relative}
  .step .ic{font-size:20px}
  .step .t{font-weight:650; font-size:14px; margin-top:6px}
  .step .d{color:var(--muted); font-size:11.5px; margin-top:2px}
  .step:not(:last-child)::after{content:"→"; position:absolute; right:-9px; top:50%; transform:translateY(-50%); color:var(--line); font-size:16px; z-index:2}
  .tags{display:flex; gap:8px; flex-wrap:wrap; margin:0 0 26px}
  .tag{font-size:12px; color:var(--muted); border:1px solid var(--line); border-radius:999px; padding:5px 11px; background:rgba(255,255,255,.02)}
  .tag b{color:var(--txt); font-weight:600}
  .tag.wa{color:var(--wa); border-color:rgba(37,211,102,.35); background:rgba(37,211,102,.08)}
  .api{border-top:1px solid var(--line); padding-top:18px}
  .api h3{font-size:12px; text-transform:uppercase; letter-spacing:.12em; color:var(--muted); margin:0 0 10px}
  .ep{font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:13px; color:#cdd9f0; padding:6px 0; display:flex; gap:10px; align-items:center}
  .ep .desc{color:#6b7da0}
  .m{font-weight:700; font-size:11px; padding:2px 7px; border-radius:6px; background:#1a2540}
  .m.get{background:rgba(110,168,254,.15); color:#9cc0ff}
  .m.post{background:rgba(34,197,94,.15); color:#7ee2a8}
  .foot{margin-top:24px; color:var(--muted); font-size:12.5px; display:flex; justify-content:space-between; flex-wrap:wrap; gap:8px}
</style>
</head>
<body>
  <main class="card">
    <div class="row">
      <span class="badge"><span class="dot"></span> <span data-es="En línea" data-en="Online">En línea</span></span>
      <span class="tag"><span data-es="Canal" data-en="Channel">Canal</span>: <b>WhatsApp</b></span>
      <span class="tag"><span data-es="Motor" data-en="Engine">Motor</span>: <b>Claude · __MODEL__</b></span>
      <span class="langtoggle" role="group" aria-label="Language">
        <button type="button" class="lang-btn" data-lang="es">ES</button>
        <button type="button" class="lang-btn" data-lang="en">EN</button>
      </span>
    </div>

    <h1>__AGENT__ <span>·</span> Asistente Comercial</h1>
    <p class="sub">__BUSINESS__</p>
    <p class="summary">__SUMMARY__</p>

    <div class="pipe">
      <div class="step"><div class="ic">💬</div>
        <div class="t" data-es="Captura" data-en="Capture">Captura</div>
        <div class="d" data-es="Responde al instante" data-en="Replies instantly">Responde al instante</div></div>
      <div class="step"><div class="ic">🎯</div>
        <div class="t" data-es="Califica" data-en="Qualify">Califica</div>
        <div class="d" data-es="Puntaje hot/warm/cold" data-en="Scores hot/warm/cold">Puntaje hot/warm/cold</div></div>
      <div class="step"><div class="ic">📅</div>
        <div class="t" data-es="Agenda" data-en="Book">Agenda</div>
        <div class="d" data-es="Cita en calendario" data-en="Calendar booking">Cita en calendario</div></div>
      <div class="step"><div class="ic">📨</div>
        <div class="t" data-es="Enruta" data-en="Route">Enruta</div>
        <div class="d" data-es="Lead al asesor" data-en="Lead to the rep">Lead al asesor</div></div>
    </div>

    <div class="tags">
      <span class="tag wa">● WhatsApp Cloud API</span>
      <span class="tag">Google Calendar</span>
      <span class="tag">HubSpot CRM</span>
      <span class="tag">Postgres</span>
      <span class="tag">Rust · axum</span>
    </div>

    <div class="api">
      <h3 data-es="Endpoints" data-en="Endpoints">Endpoints</h3>
      <div class="ep"><span class="m get">GET</span> /health</div>
      <div class="ep"><span class="m get">GET</span> /webhook <span class="desc" data-es="— verificación del webhook de Meta" data-en="— Meta webhook verification">— verificación del webhook de Meta</span></div>
      <div class="ep"><span class="m post">POST</span> /webhook <span class="desc" data-es="— mensajes entrantes de WhatsApp" data-en="— incoming WhatsApp messages">— mensajes entrantes de WhatsApp</span></div>
      <div class="ep"><span class="m get">GET</span> /conversations/&lt;id&gt;/transcript</div>
    </div>

    <div class="foot">
      <span>Asistente Comercial · MVP</span>
      <span>qualify → score → book → route</span>
    </div>
  </main>

  <script>
  (function(){
    var KEY='ac_lang';
    var els = document.querySelectorAll('[data-es]');
    function apply(l){
      if(l!=='en'){ l='es'; }
      document.documentElement.lang = l;
      for(var i=0;i<els.length;i++){
        var v = els[i].getAttribute('data-'+l);
        if(v!==null){ els[i].textContent = v; }
      }
      var btns = document.querySelectorAll('.lang-btn');
      for(var j=0;j<btns.length;j++){
        btns[j].classList.toggle('active', btns[j].getAttribute('data-lang')===l);
      }
      try{ localStorage.setItem(KEY,l); }catch(e){}
    }
    var saved=null;
    try{ saved = localStorage.getItem(KEY); }catch(e){}
    var def = (navigator.language||'es').toLowerCase().indexOf('en')===0 ? 'en' : 'es';
    var btns = document.querySelectorAll('.lang-btn');
    for(var k=0;k<btns.length;k++){
      btns[k].addEventListener('click', function(ev){ apply(ev.currentTarget.getAttribute('data-lang')); });
    }
    apply(saved || def);
  })();
  </script>
</body>
</html>"#;
