"""Synthesize the brag soundtrack: music + SFX on one 120 BPM grid counted from t=0.

Chords per bar (2 s): Am | F | C | G | Am | F | C | G | C | C   (A minor / C major)
"""
import numpy as np, soundfile as sf

SR = 44100
BPM = 180
BEAT = 60 / BPM            # 0.5 s
DUR = 12.0
N = int(SR * DUR)
rng = np.random.default_rng(7)

def midi(m): return 440.0 * 2 ** ((m - 69) / 12)
NOTE = dict(C=0, D=2, E=4, F=5, G=7, A=9, B=11)
def n(name, octave): return midi(12 * (octave + 1) + NOTE[name])

# bar -> (root(bass), chord tones for plucks (low->high), pad tones)
CH = {
    'Am': (n('A', 2), [n('A', 3), n('C', 4), n('E', 4), n('A', 4)], [n('A', 3), n('E', 4), n('C', 5)]),
    'F':  (n('F', 2), [n('F', 3), n('A', 3), n('C', 4), n('F', 4)], [n('F', 3), n('C', 4), n('A', 4)]),
    'C':  (n('C', 3), [n('C', 4), n('E', 4), n('G', 4), n('C', 5)], [n('C', 4), n('G', 4), n('E', 5)]),
    'G':  (n('G', 2), [n('G', 3), n('B', 3), n('D', 4), n('G', 4)], [n('G', 3), n('D', 4), n('B', 4)]),
}
PROG = ['Am', 'F', 'C', 'G', 'Am', 'F', 'C', 'G', 'C', 'C']
BAR = 4 * BEAT

L = np.zeros(N); R = np.zeros(N)

def put(sig, t, gain=1.0, pan=0.0):
    i = int(round(t * SR))
    if i >= N: return
    seg = sig[: N - i]
    gl = gain * np.cos((pan + 1) * np.pi / 4)
    gr = gain * np.sin((pan + 1) * np.pi / 4)
    L[i:i + len(seg)] += seg * gl
    R[i:i + len(seg)] += seg * gr

def tt(d): return np.arange(int(SR * d)) / SR

def pluck(f, d=0.6, tau=0.16, bright=1.0):
    t = tt(d)
    env = np.minimum(t / 0.004, 1) * np.exp(-t / tau)
    s = np.sin(2 * np.pi * f * t) + 0.32 * bright * np.exp(-t / (tau * .6)) * np.sin(2 * np.pi * 2 * f * t) \
        + 0.12 * bright * np.exp(-t / (tau * .35)) * np.sin(2 * np.pi * 3 * f * t)
    return s * env

def pad(freqs, d):
    t = tt(d)
    a, r = 0.7, 0.7
    env = np.clip(t / a, 0, 1) * np.clip((d - t) / r, 0, 1)
    out = np.zeros_like(t)
    for f in freqs:
        for det in (-0.004, 0.004):
            ff = f * (1 + det)
            out += np.sin(2 * np.pi * ff * t) + 0.25 * np.sin(2 * np.pi * 2 * ff * t)
    return out * env / (len(freqs) * 2)

def kick():
    t = tt(0.32)
    f = 46 + 80 * np.exp(-t * 32)
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.sin(ph) * np.exp(-t * 11) * np.minimum(t / 0.002, 1)

def hat():
    t = tt(0.07)
    x = rng.standard_normal(len(t))
    x = np.diff(x, prepend=0)  # crude high-pass
    x = np.diff(x, prepend=0)
    return x * np.exp(-t / 0.016) / 4

def whoosh(d, f0, f1, rise=True):
    k = int(SR * d); t = np.linspace(0, 1, k)
    x = rng.standard_normal(k)
    fc = f0 * (f1 / f0) ** t
    a = 1 - np.exp(-2 * np.pi * fc / SR)
    y = np.zeros(k); s = 0.0; s2 = 0.0
    for i in range(k):               # two cascaded one-pole low-passes
        s += a[i] * (x[i] - s); s2 += a[i] * (s - s2); y[i] = s2
    env = np.sin(np.pi * t) ** 2 if not rise else t ** 2.2 * (1 - np.clip((t - .92) / .08, 0, 1))
    y *= env
    return y / (np.abs(y).max() + 1e-9)

def sub(d=0.9, f=n('A', 1)):
    t = tt(d)
    return np.sin(2 * np.pi * f * t) * np.exp(-t / 0.32) * np.minimum(t / 0.005, 1)

# ---------- music (144 BPM: 10 frames per beat at 24 fps) ----------
PROG = ['Am', 'F', 'C', 'G', 'Am', 'F', 'C', 'C', 'C']
OUT_BEAT = 28                                   # title slam at beat 40 (bar 10)

def clap():
    t = tt(0.16)
    x = rng.standard_normal(len(t))
    x = x - np.convolve(x, np.ones(6) / 6, mode='same')   # crude band-pass
    return x * np.exp(-t / 0.045) * np.minimum(t / 0.002, 1)

def crash(d=1.6):
    t = tt(d)
    x = np.diff(rng.standard_normal(len(t)), prepend=0)
    x = np.diff(x, prepend=0)
    return x * np.exp(-t / 0.5) / 4

for bar, name in enumerate(PROG):
    t0 = bar * BAR
    root, tones, padt = CH[name]
    put(pad(padt, BAR + 0.5), t0, 0.15, -0.05)
    put(pad([f * 2 for f in padt[:2]], BAR + 0.5), t0, 0.05, 0.25)
    in_outro = bar * 4 >= OUT_BEAT
    if not in_outro:
        # 16th-note arpeggio
        order = [0, 1, 2, 3, 2, 1, 2, 3, 0, 2, 3, 2, 1, 2, 3, 2]
        for k in range(16):
            t = t0 + k * BEAT / 4
            put(pluck(tones[order[k]], 0.35, 0.09, bright=0.8), t, 0.10 * (1.0 if k % 4 == 0 else 0.65), pan=-0.4 + 0.8 * (k % 2))
        # pulsing 8th bass with an octave pop
        for k in range(8):
            t = t0 + k * BEAT / 2
            f = root * (2 if k % 4 == 3 else 1)
            put(pluck(f, 0.4, 0.12, bright=0.2), t, 0.30 if k % 2 == 0 else 0.20)
    else:
        for k in range(8):
            t = t0 + k * BEAT / 2
            put(pluck(tones[[0, 1, 2, 3, 2, 1, 2, 1][k]], 0.7, 0.2), t, 0.15, pan=-0.3 + 0.6 * (k % 2))
        put(pluck(root, 1.8, 0.8, bright=0.2), t0, 0.32)

# drums: kick on every beat, clap on 2 & 4, hats on the off-8ths; one-beat drop before the title
for b in range(0, OUT_BEAT - 1):
    put(kick(), b * BEAT, 0.40)
    if b % 4 in (1, 3):
        put(clap(), b * BEAT, 0.20, pan=0.1)
    put(hat(), b * BEAT + BEAT / 2, 0.22, pan=0.35)
    if b % 2 == 1:
        put(hat(), b * BEAT + BEAT / 4, 0.10, pan=-0.3)

# riser (beats 36-40) and the title impact
put(whoosh(4 * BEAT, 400, 8000, True), (OUT_BEAT - 4) * BEAT, 0.16, pan=0.0)
put(kick(), OUT_BEAT * BEAT, 0.50)
put(sub(1.8), OUT_BEAT * BEAT, 0.42)
put(crash(1.8), OUT_BEAT * BEAT, 0.28)

# ---------- SFX: every slam word, every cut ----------
PENTA = [n('A', 5), n('C', 6), n('E', 6), n('G', 6), n('D', 6)]
HITS = [0, 4, 6, 12, 16, 21.5, 24.5, 28, 28.5, 29.5, 30, 31]
for i, hb in enumerate(HITS):
    t = hb * BEAT
    put(pluck(PENTA[i % len(PENTA)], 0.4, 0.10), t, 0.11, pan=(-0.4, 0.4)[i % 2])
    put(sub(0.22, n('A', 1)), t, 0.26)               # thump under each slam
    put(clap(), t, 0.10)                              # snap
for cb in (3, 6, 9, 15, 18, 24):
    put(whoosh(0.22, 1200, 6000, False), cb * BEAT - 0.17, 0.10, pan=0.1)

# ---------- master ----------
fade = np.ones(N)
fi = int(0.015 * SR)                          # tiny anti-click fade in
fade[:fi] = np.linspace(0, 1, fi)
fo = int(0.8 * SR)
fade[-fo:] *= np.linspace(1, 0, fo) ** 1.6    # gentle end fade (last 0.8 s)
L *= fade; R *= fade

# simple glue: gentle tanh saturation then peak-normalize to -1 dBFS
def glue(x): return np.tanh(1.9 * x) / np.tanh(1.9)
L, R = glue(L), glue(R)
peak = max(np.abs(L).max(), np.abs(R).max())
g = 10 ** (-3 / 20) / peak
L *= g; R *= g
out = np.stack([L, R], 1).astype(np.float32)
sf.write('music.wav', out, SR, subtype='PCM_16')
rms = 20 * np.log10(np.sqrt((out ** 2).mean()))
print(f'wrote music.wav  {DUR}s  peak {20*np.log10(np.abs(out).max()):.1f} dBFS  rms {rms:.1f} dBFS')
