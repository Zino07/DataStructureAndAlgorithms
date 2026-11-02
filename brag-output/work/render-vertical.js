const { chromium } = require('playwright');
const { spawn } = require('child_process');
const fs = require('fs');
const FPS=30, DUR=20, POSTER_T=2.15;
(async()=>{
  const ff = spawn('ffmpeg',['-y','-loglevel','error','-f','image2pipe','-framerate',String(FPS),'-c:v','mjpeg','-i','-',
    '-c:v','libx264','-preset','slow','-crf','14','-pix_fmt','yuv420p','-r',String(FPS),'-an','video-vertical-silent.mp4'],{stdio:['pipe','inherit','inherit']});
  const b=await chromium.launch(); const p=await b.newPage({viewport:{width:1080,height:1920}});
  await p.goto('http://127.0.0.1:8766/video-vertical.html'); await p.evaluate(()=>window.ready);
  const total=FPS*DUR;
  for(let i=0;i<total;i++){
    const t = i===0 ? POSTER_T : i/FPS;      // frame 0 = poster frame (replaces, doesn't add)
    await p.evaluate(t=>render(t),t);
    const buf = await p.screenshot({type:'jpeg',quality:97});
    if(i===0) fs.writeFileSync('poster-vertical.jpg',buf);
    if(!ff.stdin.write(buf)) await new Promise(r=>ff.stdin.once('drain',r));
    if(i%60===0) console.log('frame',i,'/',total);
  }
  ff.stdin.end(); await new Promise(r=>ff.on('close',r)); await b.close(); console.log('done');
})();
