"""Soundtrack for the Aurora ad: 144 BPM (beat = 10 frames @24fps), 48 beats = 20 s, A minor.
Bar = 4 beats. Bars: 0 grey muzak | 1 Am9 (drop) | 2 Fmaj7 | 3 Cmaj9 | 4 G6/9 | 5 Am9 | 6 Fmaj7 | 7 Cmaj9 | 8 G6/9 | 9 Am9 | 10 Fmaj7 | 11 Cmaj9
"""
import numpy as np, soundfile as sf
SR=44100; BPM=144; BEAT=60/BPM; DUR=20.0; N=int(SR*DUR)
rng=np.random.default_rng(5)
def b(x): return x*BEAT
def midi(m): return 440.0*2**((m-69)/12)
NT=dict(C=0,D=2,E=4,F=5,G=7,A=9,B=11)
def n(s,o,sh=0): return midi(12*(o+1)+NT[s]+sh)
def tt(d): return np.arange(int(SR*d))/SR
BUS={k:np.zeros((N,2)) for k in ('mus','drm','fx')}
def put(bus,sig,t,gain=1.0,pan=0.0):
    i=int(round(t*SR))
    if i>=N or i<0: return
    seg=sig[:N-i]; gl=gain*np.cos((pan+1)*np.pi/4); gr=gain*np.sin((pan+1)*np.pi/4)
    BUS[bus][i:i+len(seg),0]+=seg*gl; BUS[bus][i:i+len(seg),1]+=seg*gr
def pluck(f,d=.5,tau=.12,k=6):
    t=tt(d); env=np.minimum(t/.003,1)*np.exp(-t/tau); out=np.zeros_like(t)
    for h in range(1,k+1): out+=np.sin(2*np.pi*f*h*t)/h*np.exp(-t*h*3.0/ (tau*8))
    return out*env
def bell(f,d=1.3):
    t=tt(d); env=np.minimum(t/.002,1)*np.exp(-t/.45)
    return env*(np.sin(2*np.pi*f*t)+.5*np.exp(-t/.2)*np.sin(2*np.pi*f*2.76*t)+.25*np.exp(-t/.1)*np.sin(2*np.pi*f*5.4*t))
def padc(freqs,d,a=.3,r=.5):
    t=tt(d); env=np.clip(t/a,0,1)*np.clip((d-t)/r,0,1); out=np.zeros_like(t)
    for f in freqs:
        for det in (-.006,0,.006):
            ff=f*(1+det); out+=np.sin(2*np.pi*ff*t)+.35*np.sin(4*np.pi*ff*t)+.12*np.sin(6*np.pi*ff*t)
    return out*env/(len(freqs)*3)
def bassn(f,d=.38):
    t=tt(d); env=np.minimum(t/.004,1)*np.exp(-t/.22)
    x=np.sin(2*np.pi*f*t)+.4*np.sin(4*np.pi*f*t)+.15*np.sin(6*np.pi*f*t)
    return np.tanh(1.6*x)*env
def kick():
    t=tt(.38); f=42+110*np.exp(-t*34); ph=2*np.pi*np.cumsum(f)/SR
    return np.sin(ph)*np.exp(-t*8.5)*np.minimum(t/.002,1)
def hat(d=.05):
    t=tt(d); x=rng.standard_normal(len(t)); x=np.diff(np.diff(x,prepend=0),prepend=0)
    return x*np.exp(-t/.011)/4
def ohat(): 
    t=tt(.25); x=rng.standard_normal(len(t)); x=np.diff(np.diff(x,prepend=0),prepend=0); return x*np.exp(-t/.07)/4
def clap():
    t=tt(.28); x=rng.standard_normal(len(t)); x=np.diff(x,prepend=0); env=np.zeros_like(t)
    for o in (0,.011,.022): env+=np.exp(-np.clip(t-o,0,None)/.006)*(t>=o)
    env+=.6*np.exp(-t/.09); return x*env/3
def sweep(d,f0,f1,rise=True):
    k=int(SR*d); u=np.linspace(0,1,k); x=rng.standard_normal(k); fc=f0*(f1/f0)**u
    a=1-np.exp(-2*np.pi*fc/SR); y=np.zeros(k); s=s2=0.0
    for i in range(k): s+=a[i]*(x[i]-s); s2+=a[i]*(s-s2); y[i]=s2
    env=u**2.0*(1-np.clip((u-.95)/.05,0,1)) if rise else np.sin(np.pi*u)**2
    y*=env; return y/(np.abs(y).max()+1e-9)
def crash(d=2.4):
    t=tt(d); x=rng.standard_normal(len(t)); x=np.diff(x,prepend=0); return x*np.exp(-t/.65)*np.minimum(t/.003,1)/3
def boom(d=1.5,f=n('A',1)):
    t=tt(d); fs=f*(1+1.6*np.exp(-t*16)); ph=2*np.pi*np.cumsum(fs)/SR
    return np.sin(ph)*np.exp(-t/.5)*np.minimum(t/.004,1)
def tick(f,d=.06):
    t=tt(d); return np.sin(2*np.pi*f*t)*np.exp(-t/.012)*np.minimum(t/.001,1)
def wood(): 
    t=tt(.08); return (np.sin(2*np.pi*900*t)+.5*np.sin(2*np.pi*1370*t))*np.exp(-t/.015)

CH={'Am9':(n('A',1),[n('A',3),n('C',4),n('E',4),n('G',4),n('B',4)]),
    'Fmaj7':(n('F',1),[n('F',3),n('A',3),n('C',4),n('E',4),n('G',4)]),
    'Cmaj9':(n('C',2),[n('C',4),n('E',4),n('G',4),n('B',4),n('D',5)]),
    'G69':(n('G',1),[n('G',3),n('B',3),n('D',4),n('E',4),n('A',4)])}
PROG={1:'Am9',2:'Fmaj7',3:'Cmaj9',4:'G69',5:'Am9',6:'Fmaj7',7:'Cmaj9',8:'G69',9:'Am9',10:'Fmaj7',11:'Cmaj9'}
KICKS=[]

# ---- bar 0: grey "muzak" (flat, dry, a little off) ----
for k in range(4):
    put('fx',wood(),b(k),.10,pan=.2)
for t0 in (b(0),b(2)):
    put('mus',padc([n('C',4),n('E',4),n('G',4),n('B',4)],1.1,a=.02,r=.5)*0.5,t0,.07)
# riser + reverse crash into the drop, then a hard gap before it
put('fx',sweep(1.05,500,9000),b(4)-1.12,.16)
put('fx',crash(1.0)[::-1],b(4)-1.0,.10)
# ---- THE DROP at beat 4 ----
put('fx',boom(1.6,n('A',1)),b(4),.75); put('fx',crash(2.2),b(4),.20,pan=.1); put('fx',tick(2600,.03),b(4),.4)
chime=[n('A',6),n('C',7),n('E',7),n('G',7)]
for i,bb in enumerate((4,5,6,7)):
    put('fx',bell(chime[i]),b(bb),.09,pan=(-.35,.35)[i%2]); put('fx',bell(chime[i]/2),b(bb),.05)

for bar in range(1,12):
    name=PROG[bar]; root,tones=CH[name]; t0=b(4*bar)
    # pad (sustained, ducked later)
    if bar<=9: put('mus',padc(tones[:4],BEAT*4+.5),t0,.20 if bar>1 else .22)
    elif bar==10: put('mus',padc(tones[:5],BEAT*8+.7,a=.6,r=1.2),t0,.26)
    # bass (not in outro)
    if bar<=9:
        pat=[0,2.5,3.5] if bar%2 else [0,1.5,3]
        for p in pat:
            tb=t0+b(p)
            if bar in (2,3) and b(10.5)<=tb<b(11): continue
            put('mus',bassn(root*(2 if p==2.5 else 1)),tb,.34)
    # 16th arp
    if bar<=9:
        order=[0,2,4,2,1,3,4,3, 0,2,3,2,1,3,2,1]
        dens=2 if bar in (2,3) else 1               # eighths in the "question" scene
        for k in range(16):
            if dens==2 and k%2: continue
            tk=t0+k*BEAT/4
            if b(10)<=tk<b(11) and bar==3: continue
            f=tones[order[k]%len(tones)]*2
            put('mus',pluck(f,.28,.09),tk,.075*(1 if k%4==0 else .65),pan=-.45+.9*((k//1)%2))
    if bar>=5 and bar<=9:                           # high lead
        for k,(deg,len_) in enumerate([(4,1),(3,1),(2,2)]):
            put('mus',pluck(tones[deg]*4,.5,.2),t0+b(k*1.0+.5 if bar%2 else k*1.0),.045,pan=.3)
# outro chord stack + bells
put('mus',padc([n('A',3),n('E',4),n('B',4),n('G',4),n('C',5)],BEAT*8+.8,a=.15,r=1.5),b(40),.26)
put('mus',bassn(n('F',1),1.2),b(40),.36); put('mus',bassn(n('C',2),1.2),b(44),.30)
for k,f in enumerate([n('A',5),n('C',6),n('E',6),n('G',6),n('B',6),n('E',7)]):
    put('fx',bell(f,1.4),b(41)+k*BEAT/2,.045,pan=-.4+.16*k)

# drums
def hk(t,g=.4):
    KICKS.append(t); put('drm',kick(),t,g)
for beat in range(4,48):
    t=b(beat)
    if 40<=beat: 
        if beat in (40,42,44,46): hk(t,.40 if beat==40 else .28)
        if beat in (41,43,45,47): put('drm',hat(),t,.08,pan=.3)
        continue
    if beat in (11,) : pass
    if beat in (10,): continue                       # break before the answer
    hk(t,.46)
    put('drm',hat(),t+BEAT/2,.15,pan=.3)
    if beat%2==1 and beat!=11: put('drm',clap(),t,.20,pan=-.05)
    if beat>=16 and beat<40:
        put('drm',hat(.03),t+BEAT/4,.07,pan=-.3); put('drm',hat(.03),t+3*BEAT/4,.07,pan=-.3)
    if beat>=28 and beat<40 and beat%2==0: put('drm',ohat(),t+BEAT/2,.10,pan=.3)
hk(b(11),.55)

# ---- SFX on the grid ----
for bb in (8,16,20,28):                              # scene cuts
    put('fx',sweep(.4,900,6000,False),b(bb)-.3,.10,pan=.1); put('fx',boom(.5,n('A',1)),b(bb),.22)
put('fx',boom(1.0,n('A',1)),b(11),.55); put('fx',crash(1.2),b(11),.08)
# answer counter 0 -> 256.52 (9 frames of rising ticks) then a "ding"
for i in range(9): put('fx',tick(900*2**(i/9*1.2)),b(11)+i/24,.18,pan=-.2+.05*i)
put('fx',bell(n('E',6),1.5),b(11)+9/24,.12); put('fx',bell(n('E',7),1.2),b(11)+9/24,.06)
# earned (rising, bright) and spent (falling, darker)
for i in range(9): put('fx',tick(1100*2**(i/9*1.3)),b(16)+i/24,.17,pan=.2)
put('fx',bell(n('A',6),1.5),b(16)+9/24,.12)
for i in range(9): put('fx',tick(1500*2**(-i/9*1.1)),b(18)+i/24,.17,pan=-.2)
put('fx',bell(n('E',5),1.5),b(18)+9/24,.12)
put('fx',boom(.8,n('D',1)),b(18)+9/24,.22)
for bb in (21,23,25):                                # cards fly in
    put('fx',sweep(.3,700,5000,False),b(bb)-.12,.08)
# exploded view: four cards on the beat, then accent swaps
for i,bb in enumerate((28,29,30,31)): put('fx',sweep(.28,800,6500,False),b(bb)-.08,.09,pan=-.3+.2*i); put('fx',tick(1800+200*i,.05),b(bb)+.1,.1)
for i,bb in enumerate((31,34,37)): put('fx',bell(chime[i+1]),b(bb),.09,pan=-.2+.2*i); put('fx',boom(.5,n('A',1)),b(bb),.18)
# outro
put('fx',sweep(1.3,400,11000),b(40)-1.35,.18)
put('fx',crash(2.8),b(40),.22,pan=-.1); put('fx',boom(1.8,n('F',1)),b(40),.65)
for i in range(4): put('fx',bell(chime[i],1.4),b(43+i),.07,pan=-.45+.3*i)

# sidechain pump on the music bus
duck=np.ones(N)
for kt in KICKS:
    i=int(kt*SR); m=min(N,i+int(.45*SR)); x=np.arange(m-i)/SR
    duck[i:m]*=1-.55*np.exp(-x/.11)
BUS['mus']*=duck[:,None]
mix=BUS['mus']*1.0+BUS['drm']*1.0+BUS['fx']*1.0
fade=np.ones(N); fi=int(.005*SR); fade[:fi]=np.linspace(0,1,fi); fo=int(1.2*SR); fade[-fo:]*=np.linspace(1,0,fo)**1.5
mix*=fade[:,None]
mix=np.tanh(1.7*mix)/np.tanh(1.7); peak=np.abs(mix).max(); mix*=10**(-1.5/20)/peak
sf.write('music.wav',mix.astype(np.float32),SR,subtype='PCM_16'); print('ok')
