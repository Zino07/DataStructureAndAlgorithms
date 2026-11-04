const { chromium } = require('playwright');
(async()=>{
  const ts=process.argv.slice(2).map(Number);
  const b=await chromium.launch(); const p=await b.newPage({viewport:{width:1080,height:1920}});
  p.on('pageerror',e=>console.log('pageerror',e.message)); p.on('console',m=>console.log('console',m.text()));
  await p.goto('http://127.0.0.1:8771/video.html'); await p.evaluate(()=>window.ready);
  for(const t of ts){ await p.evaluate(t=>render(t),t); await p.screenshot({path:`/tmp/claude-0/sc/s_${t.toFixed(2)}.jpg`,type:'jpeg',quality:85}); }
  await b.close();
})();
