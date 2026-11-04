"""Soundtrack: music + SFX on one 120 BPM grid (beat = 0.5 s = 12 frames @24fps), 20 s.
Bars (2 s): Gmaj7 | A | F#m7 | Bm7 | Gmaj7 | A | F#m7 | Bm7 | Gmaj7 | D
"""
import numpy as np, soundfile as sf
SR=44100; BPM=120; BEAT=60/BPM; BAR=4*BEAT; DUR=20.0; N=int(SR*DUR)
rng=np.random.default_rng(11)
def midi(m): return 440.0*2**((m-69)/12)
NOTE=dict(C=0,D=2,E=4,F=5,G=7,A=9,B=11)
def n(s,o,sh=0): return midi(12*(o+1)+NOTE[s]+sh)
L=np.zeros(N); R=np.zeros(N)
def put(sig,t,gain=1.0,pan=0.0):
    i=int(round(t*SR))
    if i>=N or i<0: return
    seg=sig[:N-i]; gl=gain*np.cos((pan+1)*np.pi/4); gr=gain*np.sin((pan+1)*np.pi/4)
    L[i:i+len(seg)]+=seg*gl; R[i:i+len(seg)]+=seg*gr
def tt(d): return np.arange(int(SR*d))/SR
def lp(x,fc):
    a=1-np.exp(-2*np.pi*fc/SR); y=np.zeros_like(x); s=0.0
    for i in range(len(x)): s+=a*(x[i]-s); y[i]=s
    return y
def pluck(f,d=.6,tau=.16,bright=1.0):
    t=tt(d); env=np.minimum(t/.004,1)*np.exp(-t/tau)
    return env*(np.sin(2*np.pi*f*t)+.32*bright*np.exp(-t/(tau*.6))*np.sin(4*np.pi*f*t)+.12*bright*np.exp(-t/(tau*.35))*np.sin(6*np.pi*f*t))
def bell(f,d=1.4):
    t=tt(d); env=np.minimum(t/.002,1)*np.exp(-t/.45)
    return env*(np.sin(2*np.pi*f*t)+.5*np.exp(-t/.2)*np.sin(2*np.pi*f*2.76*t)+.25*np.exp(-t/.1)*np.sin(2*np.pi*f*5.4*t))
def padc(freqs,d,a=.5,r=.6):
    t=tt(d); env=np.clip(t/a,0,1)*np.clip((d-t)/r,0,1); out=np.zeros_like(t)
    for f in freqs:
        for det in (-.005,0,.005):
            ff=f*(1+det); out+=np.sin(2*np.pi*ff*t)+.3*np.sin(4*np.pi*ff*t)+.1*np.sin(6*np.pi*ff*t)
    return out*env/(len(freqs)*3)
def kick(g=1):
    t=tt(.35); f=44+95*np.exp(-t*30); ph=2*np.pi*np.cumsum(f)/SR
    return np.sin(ph)*np.exp(-t*9)*np.minimum(t/.002,1)
def hat(d=.06):
    t=tt(d); x=rng.standard_normal(len(t)); x=np.diff(np.diff(x,prepend=0),prepend=0)
    return x*np.exp(-t/.014)/4
def clap():
    t=tt(.25); x=rng.standard_normal(len(t)); x=np.diff(x,prepend=0)
    env=np.zeros_like(t)
    for o in (0,.011,.022): env+=np.exp(-np.clip(t-o,0,None)/.006)*(t>=o)
    env+=.6*np.exp(-t/.08)
    return x*env/3
def noise_sweep(d,f0,f1,shape='rise'):
    k=int(SR*d); u=np.linspace(0,1,k); x=rng.standard_normal(k); fc=f0*(f1/f0)**u
    a=1-np.exp(-2*np.pi*fc/SR); y=np.zeros(k); s=s2=0.0
    for i in range(k): s+=a[i]*(x[i]-s); s2+=a[i]*(s-s2); y[i]=s2
    env=u**2.2*(1-np.clip((u-.94)/.06,0,1)) if shape=='rise' else np.sin(np.pi*u)**2
    y*=env; return y/(np.abs(y).max()+1e-9)
def crash(d=2.5):
    t=tt(d); x=rng.standard_normal(len(t)); x=np.diff(x,prepend=0)
    return x*np.exp(-t/.7)*np.minimum(t/.003,1)/3
def sub(d=1.2,f=n('G',1)):
    t=tt(d); fsw=f*(1+1.5*np.exp(-t*18)); ph=2*np.pi*np.cumsum(fsw)/SR
    return np.sin(ph)*np.exp(-t/.45)*np.minimum(t/.004,1)
def click():
    t=tt(.03); return np.sin(2*np.pi*2400*t)*np.exp(-t/.004)

# chords
CH={'Gmaj7':(n('G',2),[n('G',3),n('B',3),n('D',4),n('F',4,1)]),
    'A':    (n('A',2),[n('A',3),n('C',4,1),n('E',4),n('A',4)]),
    'F#m7': (n('F',2,1),[n('F',3,1),n('A',3),n('C',4,1),n('E',4)]),
    'Bm7':  (n('B',2),[n('B',3),n('D',4),n('F',4,1),n('A',4)]),
    'D':    (n('D',3),[n('D',4),n('F',4,1),n('A',4),n('D',5)])}
PROG=['Gmaj7','A','F#m7','Bm7','Gmaj7','A','F#m7','Bm7','Gmaj7','D']

# beat 0: music box (the light, quiet "before")
for k,f in enumerate([n('B',5),n('D',6)]):
    put(bell(f,1.0),k*BEAT/2,.06,pan=-.3+.6*k)

T_SW=1*BEAT           # the light switch
T_OUT=32*BEAT         # outro downbeat
for bar,name in enumerate(PROG):
    t0=bar*BAR; root,tones=CH[name]
    st=max(t0,T_SW)
    if bar<8:
        put(padc(tones[:3],t0+BAR+.6-st,a=.25 if bar==0 else .5),st,.13 if bar else .16)
    order=[0,1,2,3,2,1,2,3]
    for k in range(8):
        t=t0+k*BEAT/2
        if t<T_SW: continue
        if bar>=8 and k>=4 and bar==9: continue
        if 30*BEAT<=t<T_OUT: continue                     # drop before outro
        put(pluck(tones[order[k]]*2,.5,.12),t,.10*(1 if k%2==0 else .7),pan=-.35+.7*(k%2))
    if bar<8:
        for k in range(8):
            t=t0+k*BEAT/2
            if t<T_SW or 30*BEAT<=t: continue
            if k in (0,3,4,6):
                put(pluck(root,.45,.18,bright=.3),t,.30)
# outro: big held chord, then soft bell arps
put(padc([n('G',3),n('B',3),n('D',4),n('F',4,1),n('A',4)],2*BAR+.6),T_OUT,.20)
put(pluck(n('G',1)*2,2.5,.9,bright=.2),T_OUT,.35)
put(pluck(n('D',2),2.0,.8,bright=.2),T_OUT+BAR,.30)
for k,f in enumerate([n('G',5),n('B',5),n('D',6),n('F',6,1),n('A',6),n('D',6)]):
    put(bell(f,1.2),T_OUT+BAR+k*BEAT/2+BEAT,.035,pan=-.3+.12*k)

# drums
b=1
while b<30:
    t=b*BEAT
    put(kick(),t,.42)
    put(hat(),t+BEAT/2,.16,pan=.3)
    if b>=8 and b%2==1: put(clap(),t,.20,pan=-.05)
    if b>=14: put(hat(.03),t+BEAT/4,.07,pan=-.3); put(hat(.03),t+3*BEAT/4,.07,pan=-.3)
    b+=1
# outro drums: gentle kick on the downbeats
for bb in (32,34,36):
    put(kick(),bb*BEAT,.38 if bb==32 else .26)
    put(hat(),bb*BEAT+BEAT/2,.10,pan=.3); put(hat(),bb*BEAT+3*BEAT/2,.10,pan=.3)

# ---- SFX on the grid ----
put(click(),T_SW-.004,.35); put(sub(1.4),T_SW,.55); put(crash(1.8),T_SW,.10,pan=.15)
put(noise_sweep(.5,800,9000),T_SW-.5,.08,pan=-.2)      # the swell into the switch
chime=[n('F',6,1),n('A',6),n('B',6),n('D',7)]
for i,bb in enumerate([1,2,4,6]):                        # accent swaps
    put(bell(chime[i]),bb*BEAT,.07,pan=(-.35,.35)[i%2])
    put(bell(chime[i]/2),bb*BEAT,.04)
for bb in (8,14,22,28):                                  # cuts
    put(noise_sweep(.42,900,5000,'mid'),bb*BEAT-.28,.07,pan=.1)
    put(sub(.5,n('G',1)),bb*BEAT,.18)
put(noise_sweep(1.0,400,11000),T_OUT-1.0,.14)            # riser into outro
put(crash(2.8),T_OUT,.14,pan=-.1); put(sub(1.8,n('G',1)),T_OUT,.6)
for i in range(4):                                       # outro accent dots
    put(bell(chime[i]),(34+i)*BEAT,.05,pan=-.45+.3*i)

# ---- master ----
fade=np.ones(N); fi=int(.01*SR); fade[:fi]=np.linspace(0,1,fi)
fo=int(1.0*SR); fade[-fo:]*=np.linspace(1,0,fo)**1.5
L*=fade; R*=fade
def glue(x): return np.tanh(1.9*x)/np.tanh(1.9)
L,R=glue(L),glue(R); peak=max(np.abs(L).max(),np.abs(R).max()); g=10**(-1.5/20)/peak
out=np.stack([L*g,R*g],1).astype(np.float32)
sf.write('music.wav',out,SR,subtype='PCM_16'); print('ok')
