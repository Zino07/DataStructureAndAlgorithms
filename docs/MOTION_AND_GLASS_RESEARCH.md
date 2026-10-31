# Motion, morphing and glass: research and plan

Status: research, checked a second time on 2026-10-31 (see "Corrections after re-check" at the end).

## How reliable is this

* **Read in full (primary):** Apple WWDC23 "Animate with springs" (10158) and "Explore SwiftUI animation" (10156), WWDC18 "Designing Fluid Interfaces" (803), WWDC24 "Enhance your UI animations and transitions" (10145), WWDC25 "Meet Liquid Glass" (219), Apple's Keynote Magic Move help page, Material Components Android motion doc, and the Slint 1.18.1 source code on disk.
* **Not readable:** Apple's HIG pages (Materials, Motion), the `GlassEffectContainer`, `matchedTransitionSource` and "Applying Liquid Glass to custom views" pages. Those render with JavaScript and came back as titles only. What I say about the morphing API (`glassEffectID`, `GlassEffectContainer`) is from general knowledge plus search summaries, and is marked *(unverified)*.
* **Secondary:** swiftui-lab.com (matchedGeometryEffect), blog posts on glassmorphism and CSS/SVG Liquid Glass reconstructions. Vendor blogs give example numbers, not standards.
* Apple has not published how Liquid Glass is rendered. Anything about its internals beyond the WWDC25 session is a community reconstruction.

## 1. How Apple's "magic move" works

"Magic move" is three different things that people mix up:

**a) Keynote Magic Move (Apple help page).** Two slides share objects. Matched objects animate position, size, rotation and opacity from slide A to slide B. Objects that exist on only one slide fade out or fade in (optional). Text can match by word or by character. The matching is by object identity, not by looking similar.

**b) SwiftUI `matchedGeometryEffect` (swiftui-lab, Majid, others).**
* Two views are paired by `id` + `namespace`. One is removed while the other is inserted, inside an animated state change.
* The inserted view *starts* with the removed view's frame (size + position) and animates to its own. The removed one does the reverse. Default `properties: .frame`.
* The frame is all that morphs. Contents do not magically morph: the two views cross-fade (opacity) while the frame animates. Corner radius, text size and image crop need extra animatable values, which is what the swiftui-lab hero examples add.
* Pitfalls: same id for unrelated views, matching only works while insert/remove is happening, overlap needs `zIndex`.

**c) iOS 18 zoom navigation transition (WWDC24 10145).**
* Source view tagged with `matchedTransitionSource(id:in:)` (can configure clip shape/background), destination with `.navigationTransition(.zoom(sourceID:in:))`. iOS 18+.
* The tapped cell morphs into the whole destination screen, so the same content stays visible across the change. It is **interactive and interruptible**: the user can grab and drag the view at the start or in the middle, and a push that is interrupted by a pop is never cancelled: the push completes immediately and the pop starts in the same run-loop turn ("the push is always converted into a pop").
* Rule for the code behind it: be ready for a transition at any time, keep transition state minimal.
* Limitation reported on the Apple forums: the zoom takes over, so inner matched geometry doesn't compose with it.

**What we copy:** one shared "morph" layer whose rectangle (x, y, width, height, corner radius) goes from the source tile to the destination frame, with source content fading out early and destination content fading in late, driven by one interruptible spring. That is the same recipe as Apple's zoom and Material's *container transform* (container grows and contents swap; we use about 300 ms open, 250 ms close).

## 2. Apple animation rules, with the numbers

**Springs (WWDC23 10158).**
* Two perceptual parameters: `duration` (pacing) and `bounce` (-1…1). Conversions: `mass = 1`, `stiffness = (2π / duration)²`, `damping = 4π·(1 − bounce) / duration` (bounce ≥ 0). See the correction below: the slide's printed form is not what the maths or Slint use.
* bounce 0 = smooth (critically damped); about 0.15 = slightly brisk tail; about 0.3 = noticeably bouncy; above about 0.4 feels exaggerated for UI. Negative = overdamped, for decaying velocity like scrolling.
* Pick duration first, then bounce. When unsure use bounce 0. Be consistent with the app's character.
* Springs accept any initial velocity, so gestures hand over smoothly. Ease-in/out curves cannot, and halt abruptly.
* **Retargeting:** an interrupted spring uses its current velocity as the start velocity towards the new target. Each property is its own physical system and can finish at a different time. Don't wait for the settling time; use the perceptual duration.

**Fluid interfaces (WWDC18 803).**
* No latency ("everything needs to respond instantly"). Don't add timers. Highlight on touch-down, confirm on touch-up, allow cancelling by dragging away.
* Gestures must be redirectable and layerable at any moment (interrupt an open, reverse a close).
* Hysteresis: a swipe starts after the touch moves **10 pt**. Keep the touch's relative offset; never snap content centre to the finger.
* Spatial consistency: what leaves to the right comes back from the right.
* Hinting: transitions grow towards the final state so the destination is predictable.
* Springs described as **damping** (100% = no overshoot) and **response**. Start at 100% damping. Add bounce only when the gesture carried momentum in that direction: in Music, tapping to open Now Playing uses 100%, swiping it away uses 80%. (No damping value is given for picture-in-picture; that example is about velocity projection.)
* Project velocity to choose the end point (where it would coast), don't snap to the nearest. Use scroll deceleration for the projection.
* Smoothness depends on frame content, not just fps: keep change between frames small, use motion stretch / blur for fast moves.
* Avoid a double-tap recogniser on a control: it delays every single tap by about half a second.

**Material motion (Android native counterpart, Material Components doc).** Easings: standard `cubic-bezier(0.2, 0, 0, 1)`, decelerate (entering) `(0, 0, 0, 1)`, accelerate (exiting) `(0.3, 0, 1, 1)`, emphasized decelerate `(0.05, 0.7, 0.1, 1)`, emphasized accelerate `(0.3, 0, 0.8, 0.15)`. Durations: short 50–200, medium 250–400, long 450–600, extra-long 700–1000 ms; longer for larger travel. Patterns: container transform (card → screen; the official Android sample uses 300 ms, and M3 says opening should be longer than closing; the 300/250 split is our choice, not a spec), shared axis (related screens), fade-through (unrelated), fade (appear/disappear, 150 ms in / 75 ms out, linear).

**Reduce Motion.** Apple's Liquid Glass session: reduced motion lowers effect intensity and disables elastic properties; reduced transparency makes glass frostier; increased contrast makes it black/white with a border. We need all three.

## 3. Frosted glass and light

**Liquid Glass, as Apple describes it (WWDC25 219).**
* Layers that work together: **lensing** (light bent and concentrated, not scattered), **highlights** from virtual light sources that follow the shape (some respond to device motion), **shadows** that get stronger over text and weaker over light backgrounds, **illumination on touch** (glows from under the fingertip and spreads to nearby glass), **adaptive tint and range** based on what is behind.
* Interaction: it flexes and lights up on touch; resting state stays quiet. It does not fade in/out, it modulates lensing. Bigger glass looks thicker: deeper shadow, more lensing, softer scatter.
* Small elements (bars) may flip light/dark; large ones (menus, sidebars) do not flip.
* **Where to use:** only the navigation/control layer floating above content. **Never** glass on glass; don't apply glass to a list or content; things placed on glass use fills and vibrancy, not another glass. Tint only primary actions. Don't mix regular and clear. Clear only over media-rich content with a dimming layer.
* Scroll edge effect: by default content dissolves gradually under glass ("soft" is our word for it); a "hard style" with a uniform edge is used under pinned accessory views such as column headers.
* Clear glass only when all three hold: it sits over media-rich content, a dimming layer won't hurt that content, and what sits on the glass is bold and bright. We stay on the regular kind.
* Morph between glass shapes in one container (`GlassEffectContainer`, `glassEffectID`, `glassEffectUnion`; spacing sets when shapes merge) *(unverified: API page unreadable)*.

**Practical recipe from the CSS/design sources (use as starting points, not standards).**
* Fill alpha about 10–40%, text layers more opaque; blur plus saturation boost to stop blur from greying colours; 1 px light border so the pane doesn't melt; soft shadow; brighter border on the top-left edge as a light source.
* Dark glass needs a **busy backdrop** (gradient blobs, image). Over a flat colour it shows nothing. Aurora: 2–4 blurred colour blobs, fine grain (about 4% opacity noise) to stop banding.
* Contrast is the main risk: 4.5:1 for body text, 3:1 for large text, tested against the **worst-case** pixel behind the pane, with a solid fallback for reduced transparency.
* Community Liquid Glass reconstructions: a displacement (refraction) map from a rounded-rect signed distance field, a specular rim from the surface normal, a Fresnel term, optional chromatic aberration at the edge only.

## 4. What our stack can do (checked in the Slint 1.18.1 source)

| Need | Available? |
|---|---|
| Spring animations | **Yes.** `animate x { duration: 500ms; easing: spring(0.15); }`, bounce -1…1, same model as Apple's duration/bounce. A retarget mid-flight carries the outgoing velocity (there is a test for it). |
| Cubic-bezier, elastic, bounce easings | Yes |
| Scale / rotate / transform-origin | Yes (not on the software renderer, we use femtovg) |
| Linear, radial and conic gradients | Yes |
| Drop shadow with blur | Yes (`drop-shadow-blur`, colour, offset) |
| Image clipping (`source-clip-x/y/width/height`) | Yes |
| Per-frame driver | `animation-tick()` |
| **Backdrop blur / real frosted glass over moving content** | **No** |
| Refraction / lensing | No |
| Single-element gradient border stroke | Path strokes take a brush; to be tested |

**Workaround for real frosting without backdrop blur.** Render one blurred "aurora" image at startup (a few radial blobs, blurred in Rust, ~360×780 px). Draw it fixed behind the whole app. Each glass panel is a tinted `Image` of that same blurred picture, clipped to the part that lies behind the panel (`source-clip-*` follows the panel's position). It looks like real frosted glass with no runtime blur. Limits: it frosts the background only, not cards or text scrolling underneath; the dock over scrolling content keeps the dense tint plus fade. Panel count and image size must be measured on a phone.

## 5. Plan (nothing started)

### A. Motion system
1. Tokens in `Theme`: `Motion.smooth` (spring 0, 450 ms), `snappy` (spring 0.15, 380 ms), `bouncy` (spring 0.3, 500 ms), `quick` (200 ms bezier 0.2,0,0,1 for fades), `enter` / `exit` bezier pairs. Rule: bounce only after a gesture with momentum.
2. Entrance: cards rise 24 px and fade in with 40 ms stagger; bars grow from the baseline; progress bars fill; amounts count up with a spring.
3. Press feedback: scale 0.97 with a small bounce, plus a soft light on glass under the finger. Highlight on touch-down, confirm on release.
4. Shared indicators: the pill-nav thumb and the dock's active dot **glide** between items (matched geometry); chips select with a sliding highlight.
5. Scroll-linked: large title collapses, top glass bar fades in with the scroll edge effect, hero parallax.
6. Ambient (optional, switchable): slow drift of the aurora (translate a pre-rendered image, almost free); radar sweep on the logo.
7. Reduce motion: in-app switch (and read the system setting if reachable) swaps springs for 150 ms fades and stops ambient loops.

### B. Morph transitions (the "magic move")
One interruptible `progress` driven by a spring (0 → 1 open, 1 → 0 close; reversing mid-way keeps velocity). Morph layer interpolates rectangle and corner radius from the source's measured geometry to the destination; source content fades out in the first ~35%, destination content fades in from ~40%; the page below scales to 0.96 and dims. Candidates:
* Total-balance tile → Accounts
* **Add button / dock "+" → New entry** (grows out of the button, returns into it)
* Recent / activity row → Edit entry
* Stat tiles → Stats detail
* Account row → account detail (new)
* Interactive back: drag from the left edge or down to set `progress` directly, release projects velocity and settles with a spring (damping ~80% when the throw has momentum).
* Screen-to-screen without a source: shared-axis slide of 30 px plus fade (Material), not a full push.

### C. "Aurora Glass" theme (separate style)
* A **Style** choice: Bento (current, solid) or **Aurora Glass**. Accent colours (Lime, Solar, Ember, Lilac) tint the aurora and glow in both.
* Full-bleed blurred aurora background from the accent (like your green reference), fine grain at about 4%.
* Panels: blurred-background crop + dark tint (about 30–45% over the aurora, raised where text sits) + 1 px edge brighter at top-left + soft inner glow + tinted shadow + faint specular line along the top edge. One light source, top-left, everywhere.
* Touch illumination: a radial glow under the finger that spreads and fades.
* Small accent glow behind hero numbers and the primary button.
* Rules from Apple that I will follow: glass for navigation/controls and a few hero panels only, never glass inside glass, tint only primary actions, no mixing of two glass types, contrast checked against the brightest part of the aurora, solid fallback when "reduce transparency" is on.
* Not possible here: refraction/lensing and blur of live content. I will say so in the app's docs rather than imply it.

### D. The 17 fixes
All 17 from the previous message stay in scope (consistent number format, denser top bar, bottom space, aligned card headers, honest 7-day bars, cash-flow chart scale, money-clock layout, category tiles, When area, day totals, smaller activity summary, add-account button, account shares, budget progress with suggested limits, accent swatches, disabled stepper style, light-theme checks).

### E. How it will be checked
* Timeline screenshots at fixed times through each morph (0, 100, 200, 300, 450 ms) in the test backend, then looked at.
* Unit tests for the interpolation maths and for "reverse mid-way never jumps".
* The 100×5 simulation re-run with animations on (it advances mock time), plus frame-time measurement on desktop.
* Honest limit: frame rate and battery on a real phone cannot be measured here. The APK is built, not run on a device.

### Risks
* Many clipped `Image` panels on a mid-range phone: may need fewer panels or a smaller background image. I will measure on desktop and say so.
* `spring()` gives no live velocity from a drag by itself; interactive back needs the progress set by hand and a spring started at release.
* Morph source rectangles must be measured at tap time; scrolling during a morph needs the layer locked to window coordinates.

## Sources

* Apple, [Animate with springs, WWDC23](https://developer.apple.com/videos/play/wwdc2023/10158/)
* Apple, [Explore SwiftUI animation, WWDC23](https://developer.apple.com/videos/play/wwdc2023/10156/)
* Apple, [Designing Fluid Interfaces, WWDC18](https://developer.apple.com/videos/play/wwdc2018/803/)
* Apple, [Enhance your UI animations and transitions, WWDC24](https://developer.apple.com/videos/play/wwdc2024/10145/)
* Apple, [Meet Liquid Glass, WWDC25](https://developer.apple.com/videos/play/wwdc2025/219/)
* Apple, [Add a Magic Move transition (Keynote)](https://help.apple.com/keynote/mac/6.6/en.lproj/tan2bdb4d136.html)
* Apple, [Liquid Glass overview](https://developer.apple.com/documentation/technologyoverviews/liquid-glass) and [HIG: Materials](https://developer.apple.com/design/human-interface-guidelines/materials) (titles only when fetched)
* Material Components Android, [Motion](https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md)
* SwiftUI Lab, [matchedGeometryEffect, part 1](https://swiftui-lab.com/matchedgeometryeffect-part1/)
* Majid, [Hero animations in SwiftUI](https://swiftwithmajid.com/2020/12/17/hero-animations-in-swiftui/)
* Create with Swift, [Zoom navigation transition](https://createwithswift.com/using-the-zoom-navigation-transition-in-swiftui)
* Ramotion, [What is glassmorphism](https://www.ramotion.com/blog/what-is-glassmorphism/); Axess Lab, [Glassmorphism meets accessibility](https://axesslab.com/?p=5111)
* LogRocket, [Liquid Glass with CSS and SVG](https://blog.logrocket.com/how-create-liquid-glass-effects-css-and-svg/); kube.io, [Liquid Glass in the browser](https://hn.jakechampion.name/reader/https://kube.io/blog/liquid-glass-css-svg)
* Slint 1.18.1 source (`i-slint-core/animations.rs`, `properties/properties_animations.rs`, `i-slint-compiler` tests) and [Slint docs](https://docs.slint.dev/latest/docs/slint/reference/common)

## Corrections after re-check (2026-10-31)

A second, lighter pass against the primary sources (WWDC23 10158, WWDC18 803, WWDC24 10145, WWDC25 219 transcripts and the Slint 1.18.1 source) found these problems in the first pass:

1. **Spring damping formula was wrong.** The first pass copied `damping = 1 − 4π·bounce / duration` as it is printed on the WWDC23 slide. That form does not hold: with bounce 0 it gives damping 1 for every duration, but critical damping for mass 1 and stiffness (2π/d)² is 4π/d. The consistent form is `damping = 4π·(1 − bounce) / duration`, i.e. natural frequency ω = 2π/duration and damping ratio ζ = 1 − bounce. Slint implements exactly this (`animations/simulations/spring.rs`: `w_n = 2π / duration`, `zeta = 1 − bounce`). For negative bounce Slint keeps ζ = 1 − bounce (overdamped), which differs a little from Apple's slide; we do not use negative bounce.
2. **Duration is perceptual, not the end time.** Apple: don't wait for the settling time, which is unpredictable. In Slint a spring keeps moving after `duration` and is forced to settle within about 10× duration. Consequence for us: screenshots and the simulator must not assume an animation is finished at `duration`; logic must never wait on an animation.
3. **Interrupted push:** it is completed immediately and converted into a pop, not "played to the end then reversed".
4. **Container transform 300/250 ms** is not an official Material number. The official sample uses 300 ms; M3 only says opening should take longer than closing.
5. **Music example**, not "mini-player"; no damping number exists for picture-in-picture.

Confirmed unchanged: bounce guidance (0 smooth, ~0.15 brisk, ~0.3 noticeably bouncy, avoid > ~0.4), 10 pt hysteresis, 100% vs 80% damping rule, velocity projection with the scroll deceleration rate, Liquid Glass rules (navigation layer only, no glass on glass, fills/vibrancy on top of glass, regular vs clear never mixed, tint only primary actions, small elements flip light/dark and big ones don't, shadow stronger over text, touch glow spreading from the fingertip to nearby glass, Reduce Transparency/Increase Contrast/Reduce Motion behaviour), Material easing curves and duration ranges.
