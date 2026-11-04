"""Synthesize the brag soundtrack: music + SFX on one 120 BPM grid counted from t=0.

Chords per bar (2 s): Am | F | C | G | Am | F | C | G | C | C   (A minor / C major)
"""
import numpy as np, soundfile as sf

SR = 44100
BPM = 120
BEAT = 60 / BPM            # 0.5 s
DUR = 20.0
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

# ---------- music ----------
for bar, name in enumerate(PROG):
    t0 = bar * BAR
    root, tones, padt = CH[name]
    # pad (overlaps into next bar)
    put(pad(padt, BAR + 0.8), t0, 0.15, -0.05)
    put(pad([f * 2 for f in padt[:2]], BAR + 0.8), t0, 0.05, 0.25)
    # arpeggio: 8ths, 1-2-3-4-3-2-1-2
    order = [0, 1, 2, 3, 2, 1, 0, 1]
    last_bar = bar >= 8
    for k in range(8):
        if last_bar and k >= 2 and not (bar == 8 and k < 6):
            continue
        t = t0 + k * BEAT / 2
        accent = 1.0 if k % 2 == 0 else 0.7
        put(pluck(tones[order[k]], 0.7, 0.17), t, 0.17 * accent, pan=-0.3 + 0.6 * (k % 2))
    # bass: from first cut (2.5s) until the outro bar
    if bar >= 1 and bar < 8:
        for b in (0, 2):
            tb = t0 + b * BEAT
            if tb >= 2.5:
                put(pluck(root, 1.0, 0.45, bright=0.2), tb, 0.30)
        tb = t0 + 3 * BEAT        # syncopated pickup on beat 4
        if tb >= 2.5 and bar % 2 == 1:
            put(pluck(root * 2, 0.6, 0.2, bright=0.2), tb + BEAT / 2 - BEAT / 2, 0.12)
    if bar >= 8:
        put(pluck(root, 1.6, 0.7, bright=0.2), t0, 0.30)

# drums: soft kick on every beat 2.5 -> 17.0, hats on the off-8ths
b = 5
while b * BEAT < 17.0:
    put(kick(), b * BEAT, 0.34)
    put(hat(), b * BEAT + BEAT / 2, 0.20, pan=0.35)
    b += 1
put(kick(), 17.0, 0.40)
put(sub(1.4), 17.0, 0.35)             # outro downbeat

# riser into the first cut and a swell into the outro
put(whoosh(0.8, 500, 7000, True), 1.7, 0.13, pan=-0.2)
put(whoosh(0.5, 500, 6000, True), 16.5, 0.13, pan=0.2)

# ---------- SFX (on the beat grid: they land exactly when each item settles in) ----------
PENTA = [n('A', 5), n('C', 6), n('E', 6), n('G', 6), n('D', 6)]
def tick(t, i, g=0.075):
    put(pluck(PENTA[i % len(PENTA)], 0.5, 0.11), t, g, pan=(-0.4, 0.4)[i % 2])

reveals = [0.0, 0.5, 1.0,                       # hook words
           2.5, 3.0, 3.5, 4.5, 5.5,             # headline, cards
           7.0, 7.5, 8.0, 8.5, 9.0, 9.5, 11.0,  # fit headline, sub, rows, caption
           13.0, 14.0, 15.0,                    # trust items
           17.5, 18.0, 18.5, 19.0]              # outro
for i, t in enumerate(reveals):
    tick(t, i)
# the cursor click on size 56 (10.5 s): lower, rounder tock
put(pluck(n('E', 4), 0.4, 0.09, bright=0.3), 10.5, 0.16)
put(pluck(n('E', 5), 0.4, 0.07), 10.5, 0.07)

# soft whooshes under each scene change (old content eases out 0.3s before the cut)
for tcut in (2.5, 7.0, 12.5):
    put(whoosh(0.5, 900, 3500, False), tcut - 0.25, 0.07, pan=0.1)
put(whoosh(0.5, 900, 3500, False), 16.75, 0.05)

# ---------- master ----------
fade = np.ones(N)
fi = int(0.015 * SR)                          # tiny anti-click fade in
fade[:fi] = np.linspace(0, 1, fi)
fo = int(0.8 * SR)
fade[-fo:] *= np.linspace(1, 0, fo) ** 1.6    # gentle end fade (last 0.8 s)
L *= fade; R *= fade

# simple glue: gentle tanh saturation then peak-normalize to -1 dBFS
def glue(x): return np.tanh(1.4 * x) / np.tanh(1.4)
L, R = glue(L), glue(R)
peak = max(np.abs(L).max(), np.abs(R).max())
g = 10 ** (-3 / 20) / peak
L *= g; R *= g
out = np.stack([L, R], 1).astype(np.float32)
sf.write('music.wav', out, SR, subtype='PCM_16')
rms = 20 * np.log10(np.sqrt((out ** 2).mean()))
print(f'wrote music.wav  {DUR}s  peak {20*np.log10(np.abs(out).max()):.1f} dBFS  rms {rms:.1f} dBFS')
