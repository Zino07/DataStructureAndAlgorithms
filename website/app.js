
(function(){
  "use strict";
  var dream = document.getElementById("dream");
  var meter = document.getElementById("meter");
  var homePanels = document.getElementById("homePanels");
  var panels = [].slice.call(document.querySelectorAll("[data-panel]"));
  var shopView = document.getElementById("shopView");
  var aboutView = document.getElementById("aboutView");
  var dashboardView = document.getElementById("dashboardView");
  var adminView = document.getElementById("adminView");
  var contactModal = document.getElementById("contactModal");
  var authModal = document.getElementById("authModal");
  var authForm = document.getElementById("authForm");
  var authEmail = document.getElementById("authEmail");
  var authBtn = document.getElementById("authBtn");
  var closeAuthBtn = document.getElementById("closeAuthBtn");
  var logoutBtn = document.getElementById("logoutBtn");
  var userGreeting = document.getElementById("userGreeting");
  var ordersInfo = document.getElementById("ordersInfo");
  var toaster = document.getElementById("toaster");
  var CUES = [[0.00,0.00,0.10,0.17],[0.24,0.30,0.40,0.46],[0.53,0.59,0.69,0.75],[0.82,0.88,1.10,1.20]];
  var DRIFT = 22;
  var reduceMotion = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
  function clamp(v,a,b){ return Math.max(a, Math.min(b,v)); }
  function smooth(t){ return t*t*(3-2*t); }
  function ramp(p,a,b){ if(b<=a) return p>=b?1:0; return smooth(clamp((p-a)/(b-a),0,1)); }

  var I18N = {
  en: { navShop:"Shop", navAbout:"About", navSignin:"Sign In", bag:"Bag", home:"Home", myNoor:"My Noor", title:"Noor — Modest Fashion Store",
    h1e:"Noor · Thobe House", h1t:"Covered <em>in grace.</em><br>Modest <em>always.</em>", h1s:"Premium thobes, djellabas and shirts — tailored Algerian cuts and breathable fabrics, ready for Friday prayer and Eid.", shopBest:"Shop bestsellers", philo:"Our philosophy",
    h2e:"Every occasion", h2t:"Modest, <em>never</em> plain.", h2s:"Thobes that carry you from the masjid to the family lunch — pressed, breathable and quietly premium.", ritual:"Find your thobe",
    h3e:"The cut", h3t:"Tailored enough to <em>last years.</em>", h3s:"Breathing crepe, linen and premium jersey — cut, finished and shipped in small weekly drops.", off10:"Get 10% off",
    carE:"Bestsellers — drag or tap", carT:"Feel the <em>fabrics.</em>", shopAll:"Shop all 11",
    shopTitle:"The Shop", shopSub:"11 essential pieces — thobes, djellabas and shirts for prayer and Eid. Genuine imports, prices in DZD. Free delivery over 8,000 DZD.", searchPh:"Search products…", all:"All", products:"products", noneFound:"No products match your search.", add:"Add", addBag:"Add to Bag", dec:"Decrease quantity", inc:"Increase quantity", loadMore:"Load more", lowLeft:function(n){ return "Only "+n+" left"; }, oos:"Out of stock", errReq:"Required", errPhone:"Invalid Algerian number (05/06/07…)",
    bagTitle:"Your Bag", bagEmpty:"Your bag is empty. Add a Noor essential to begin.", total:"Total", checkout:"Checkout", close:"Close",
    addedOne:function(n){ return n+" added to bag"; }, bagEmptyErr:"Your bag is empty",
    authTitle:"Welcome to Noor", authSub:"Sign in for faster checkout and order tracking.", email:"Email", pass:"Password", cancel:"Cancel", enter:"Enter",
    hello:function(e){ return "Welcome to Noor, "+e; }, bye:"Signed out", authErr:"Invalid email or password.", regConfirm:function(e){ return "No account for "+e+" — create one with this password?"; },
    dashTitle:"My Noor", clientWs:"Client workspace", orders:"Orders", ordersEmpty:"No orders yet — your Noor wardrobe starts in the Shop.", account:"Account", accountDesc:"Manage sizes, shipping and order preferences.", signout:"Sign Out",
    glowAcc:function(e){ return "Style credits: "+e; },
    abTitle:"Noor Philosophy", abSub:"Noor is a small-batch modest fashion house. Fewer, better pieces — honest fabrics, quiet details, no compromise.",
    ab1t:"Tailored + Tested", ab1d:"Every thobe and djellaba is cut for easy movement, with double-stitched seams that survive daily wear.",
    ab2t:"Breathing Fabrics", ab2d:"Natural crepe, linen and premium jersey. No scratchy synthetics, no see-through surprises.",
    ab3t:"Small Drops", ab3d:"Released weekly in micro-batches so every piece stays fresh and in demand.",
    coTitle:"Checkout", coName:"Full name", coNamePh:"Your name", coPhone:"Phone", coPhonePh:"05 / 06 / 07…", coWilaya:"Wilaya", coCommune:"Commune", coCommunePh:"Commune",
    coShip:"Delivery", provider:"Delivery company", coHome:"Home delivery", coStop:"Stopdesk (office pickup)", coPay:"Payment", coCod:"Cash on delivery",
    coPromo:"Promo code", coPromoPh:"NOOR10", coApply:"Apply", coPromoOk:"Promo applied: −10%", coPromoBad:"Invalid promo code",
    coSub:"Subtotal", coDisc:"Discount", coShip:"Shipping", coTotal:"Total", coPlace:"Place order",
    coOk:"Order confirmed!", coCont:"Continue shopping", coFill:"Please fill name, phone and wilaya",
    coOrderNo:function(n){ return "Order "+n+" · Cash on delivery"; },
    coPlaced:function(n,tt){ return "Order placed ("+n+(n===1?" item":" items")+", "+tt+"). Thank you."; },
    offSaved:"Code NOOR10 saved — 10% off your first ensemble.", free:"Free",
    reviews:"Reviews", noReviews:"No reviews yet — be the first.", reviewName:"Your name", reviewText:"Your review…", reviewSend:"Send", reviewThanks:"Thanks! Your review was saved.", reviewBad:"Please add a rating and a few words.", copyLink:"Copy link", copied:"Link copied", waOrder:"WhatsApp order", tooFast:"Too many requests — slow down.",
    avgLine:function(n,r){ return r+" ★ · "+n+" reviews"; }, qvNote:"Original cut · Prices in DZD · Free delivery over 8,000 DZD",
    foot:"Noor Modest Fashion &nbsp;·&nbsp; 12 Hay El Nasr &nbsp;·&nbsp; Open daily, 9am – 9pm" },
  fr: { navShop:"Boutique", navAbout:"À propos", navSignin:"Se connecter", bag:"Panier", home:"Accueil", myNoor:"Mon Noor", title:"Noor — Boutique Mode Modeste",
    h1e:"Noor · Maison du Thobe", h1t:"Vêtue <em>de grâce.</em><br>Modeste <em>toujours.</em>", h1s:"Thobes, djellabas et chemises premium — coupes algériennes taillées, étoffes respirantes, prêts pour le Vendredi et l\u2019Aïd.", shopBest:"Voir les best-sellers", philo:"Notre philosophie",
    h2e:"Pour chaque occasion", h2t:"Modeste, <em>jamais</em> banale.", h2s:"Des thobes qui vous portent de la mosquée au déjeuner familial — repassées, respirantes et discrètement premium.", ritual:"Trouver mon thobe",
    h3e:"La coupe", h3t:"Assez soignée pour <em>durer des années.</em>", h3s:"Crêpe respirant, lin et jersey premium — coupés, finis et expédiés en petits lots hebdomadaires.", off10:"−10% offerts",
    carE:"Meilleures ventes — glissez ou touchez", carT:"Sentez les <em>étoffes.</em>", shopAll:"Voir les 11",
    shopTitle:"La Boutique", shopSub:"11 pièces essentielles — thobes, djellabas et chemises pour la prière et l\u2019Aïd. Imports authentiques, prix en DZD. Livraison offerte dès 8 000 DZD.", searchPh:"Rechercher…", all:"Tout", products:"produits", noneFound:"Aucun produit ne correspond.", add:"Ajouter", addBag:"Ajouter au panier", dec:"Diminuer la quantité", inc:"Augmenter la quantité", loadMore:"Afficher plus", lowLeft:function(n){ return "Plus que "+n+" !"; }, oos:"Rupture de stock", errReq:"Requis", errPhone:"Numéro algérien invalide (05/06/07…)",
    bagTitle:"Votre panier", bagEmpty:"Votre panier est vide. Ajoutez un essentiel Noor.", total:"Total", checkout:"Commander", close:"Fermer",
    addedOne:function(n){ return n+" ajouté au panier"; }, bagEmptyErr:"Votre panier est vide",
    authTitle:"Bienvenue chez Noor", authSub:"Connectez-vous pour un paiement plus rapide et le suivi.", email:"E-mail", pass:"Mot de passe", cancel:"Annuler", enter:"Entrer",
    hello:function(e){ return "Bienvenue chez Noor, "+e; }, bye:"Déconnexion réussie", authErr:"E-mail ou mot de passe invalide.", regConfirm:function(e){ return "Pas de compte pour "+e+" — en créer un avec ce mot de passe ?"; },
    dashTitle:"Mon Noor", clientWs:"Espace client", orders:"Commandes", ordersEmpty:"Aucune commande — votre garde-robe Noor commence dans la Boutique.", account:"Compte", accountDesc:"Gérez tailles, livraison et préférences.", signout:"Se déconnecter",
    glowAcc:function(e){ return "Crédits style : "+e; },
    abTitle:"Philosophie Noor", abSub:"Noor est une maison de mode modeste en petits lots. Moins de pièces, meilleures — étoffes honnêtes, détails discrets, sans compromis.",
    ab1t:"Taillé + Éprouvé", ab1d:"Chaque thobe et djellaba est taillé pour un mouvement facile, avec coutures doubles qui durent.",
    ab2t:"Étoffes Respirantes", ab2d:"Crêpe naturel, lin et jersey premium. Sans synthétique qui gratte, ni surprise transparente.",
    ab3t:"Petites séries", ab3d:"Livré chaque semaine en micro-lots pour que chaque pièce reste fraîche.",
    coTitle:"Commande", coName:"Nom complet", coNamePh:"Votre nom", coPhone:"Téléphone", coPhonePh:"05 / 06 / 07…", coWilaya:"Wilaya", coCommune:"Commune", coCommunePh:"Commune",
    coShip:"Livraison", provider:"Transporteur", coHome:"À domicile", coStop:"Au bureau (stopdesk)", coPay:"Paiement", coCod:"Paiement à la livraison",
    coPromo:"Code promo", coPromoPh:"NOOR10", coApply:"Appliquer", coPromoOk:"Promo appliquée : −10%", coPromoBad:"Code promo invalide",
    coSub:"Sous-total", coDisc:"Remise", coShip:"Livraison", coTotal:"Total", coPlace:"Valider la commande",
    coOk:"Commande confirmée !", coCont:"Continuer mes achats", coFill:"Veuillez remplir nom, téléphone et wilaya",
    coOrderNo:function(n){ return "Commande "+n+" · Paiement à la livraison"; },
    coPlaced:function(n,tt){ return "Commande passée ("+n+(n===1?" article":" articles")+", "+tt+"). Merci."; },
    offSaved:"Code NOOR10 enregistré — −10% sur votre premier ensemble.", free:"Offert",
    reviews:"Avis", noReviews:"Aucun avis — soyez le premier.", reviewName:"Votre nom", reviewText:"Votre avis…", reviewSend:"Envoyer", reviewThanks:"Merci ! Avis enregistré.", reviewBad:"Ajoutez une note et quelques mots.", copyLink:"Copier le lien", copied:"Lien copié", waOrder:"Commander sur WhatsApp", tooFast:"Trop de requêtes — ralentissez.",
    avgLine:function(n,r){ return r+" ★ · "+n+" avis"; }, qvNote:"Coupe originale · Prix en DZD · Livraison offerte dès 8 000 DZD",
    foot:"Noor Mode Modeste &nbsp;·&nbsp; 12 Hay El Nasr &nbsp;·&nbsp; Ouvert tous les jours, 9h – 21h" },
  ar: { navShop:"المتجر", navAbout:"من نحن", navSignin:"تسجيل الدخول", bag:"السلة", home:"الرئيسية", myNoor:"نور الخاصة", title:"نور — متجر ملابس محتشمة",
    h1e:"نور · دار الثوب", h1t:"الحشمة <em>أولاً.</em><br>والأناقة <em>دائماً.</em>", h1s:"ثوب وجلابيب وقمصان برايم — قصّ جزائري متقن وأقمشة متنفسة، جاهز للجمعة والعيد.", shopBest:"تسوق الأكثر مبيعاً", philo:"فلسفتنا",
    h2e:"لكل مناسبة", h2t:"محتشمة، <em>أبداً</em> عادية.", h2s:"ثوب يرافقك من المسجد إلى غداء العائلة — مكنون، متنفس وفخم بهدوء.", ritual:"اختار ثوبك",
    h3e:"القصّة", h3t:"مُتقنة بما يكفي <em>لتبقى سنوات.</em>", h3s:"كريب متنفس، قماش وكتان وجيرسي برايم — قصّة وإنهاء وتغليف بدفعات صغيرة أسبوعياً.", off10:"خصم 10%",
    carE:"الأكثر مبيعاً — اسحب أو المس", carT:"المس <em>الأقمشة.</em>", shopAll:"تسوق الأحد عشر",
    shopTitle:"المتجر", shopSub:"11 قطعة أساسية — ثوب وجلابيب وقمصان للصلاة والعيد. استيراد أصلي، الأسعار بالدينار. توصيل مجاني فوق 8,000 دج.", searchPh:"ابحث عن منتج…", all:"الكل", products:"منتج", noneFound:"لا توجد منتجات مطابقة.", add:"أضف", addBag:"أضف إلى السلة", dec:"إنقاص الكمية", inc:"زيادة الكمية", loadMore:"عرض المزيد", lowLeft:function(n){ return "بقي "+n+" فقط!"; }, oos:"نفد المخزون", errReq:"مطلوب", errPhone:"رقم جزائري غير صالح (05/06/07…)",
    bagTitle:"سلتك", bagEmpty:"سلتك فارغة. أضف أساسياً من نور.", total:"المجموع", checkout:"إتمام الطلب", close:"إغلاق",
    addedOne:function(n){ return "تمت إضافة "+n+" إلى السلة"; }, bagEmptyErr:"سلتك فارغة",
    authTitle:"مرحباً بك في نور", authSub:"سجّل الدخول لدفع أسرع وتتبع الطلبات.", email:"البريد الإلكتروني", pass:"كلمة المرور", cancel:"إلغاء", enter:"دخول",
    hello:function(e){ return "مرحباً بك في نور، "+e; }, bye:"تم تسجيل الخروج", authErr:"البريد أو كلمة المرور غير صالحة.", regConfirm:function(e){ return "لا يوجد حساب لـ "+e+" — إنشاء حساب بهذه الكلمة؟"; },
    dashTitle:"نور الخاصة", clientWs:"مساحة العميل", orders:"الطلبات", ordersEmpty:"لا طلبات بعد — خزانة نور تبدأ من المتجر.", account:"الحساب", accountDesc:"أدر مقاساتك وشحنك وتفضيلاتك.", signout:"تسجيل الخروج",
    glowAcc:function(e){ return "نقاط الأناقة: "+e; },
    abTitle:"فلسفة نور", abSub:"نور دار أزياء محتشمة بدفعات صغيرة. قطع أقل وأفضل — أقمشة صادقة وتفاصيل هادئة، بلا مساومة.",
    ab1t:"قصّ + مُجرّب", ab1d:"كل ثوب وجلابة، مُقصّان لحركة مريحة، بدرزات مزدوجة تصمد للاستخدام اليومي.",
    ab2t:"أقمشة متنفسة", ab2d:"كريب وكتان وجيرسي طبيعي برايم. بلا صنّاعي خشن، بلا شفافية مفاجئة.",
    ab3t:"دفعات صغيرة", ab3d:"الإصدار أسبوعياً بدفعات صغيرة لتبقى كل قطعة طازجة ومطلوبة.",
    coTitle:"إتمام الطلب", coName:"الاسم الكامل", coNamePh:"اسمك", coPhone:"الهاتف", coPhonePh:"05 / 06 / 07…", coWilaya:"الولاية", coCommune:"البلدية", coCommunePh:"البلدية",
    coShip:"التوصيل", provider:"شركة التوصيل", coHome:"إلى المنزل", coStop:"إلى المكتب", coPay:"الدفع", coCod:"الدفع عند الاستلام",
    coPromo:"كود الخصم", coPromoPh:"NOOR10", coApply:"تطبيق", coPromoOk:"تم تطبيق الخصم: −10%", coPromoBad:"كود غير صالح",
    coSub:"المجموع الفرعي", coDisc:"الخصم", coShip:"الشحن", coTotal:"المجموع", coPlace:"تأكيد الطلب",
    coOk:"تم تأكيد طلبك!", coCont:"مواصلة التسوق", coFill:"يرجى ملء الاسم والهاتف والولاية",
    coOrderNo:function(n){ return "طلب "+n+" · الدفع عند الاستلام"; },
    coPlaced:function(n,tt){ return "تم تقديم الطلب ("+n+(n===1?" منتج":" منتجات")+"، \u2066"+tt+"\u2069). شكراً."; },
    offSaved:"تم حفظ الكود NOOR10 — خصم 10% على أول إطلالة.", free:"مجاني",
    reviews:"التقييمات", noReviews:"لا تقييمات بعد — كن الأول.", reviewName:"اسمك", reviewText:"رأيك…", reviewSend:"إرسال", reviewThanks:"شكراً! تم حفظ تقييمك.", reviewBad:"أضف تقييماً وبضع كلمات.", copyLink:"نسخ الرابط", copied:"تم نسخ الرابط", waOrder:"اطلب عبر واتساب", tooFast:"طلبات كثيرة — تمهل.",
    avgLine:function(n,r){ return r+" ★ · "+n+" تقييمات"; }, qvNote:"تصميم أصلي · الأسعار بالدينار · توصيل مجاني فوق 8,000 دج",
    foot:"نور للأزياء المحتشمة &nbsp;·&nbsp; 12 حي النصر &nbsp;·&nbsp; مفتوح يومياً 9ص – 9م" }
  };
  var I18N_EXTRA = {
    en: {
      forgotLink:"Forgot password?",
      forgotHelp:"Enter your account email. We'll send you a link to choose a new password.",
      back:"Back",
      forgotSend:"Send link",
      forgotSent:"If an account exists for this email, a reset link is on its way. Check your inbox and spam folder.",
      forgotNoMail:"Password reset by email isn't available yet. Message us on WhatsApp and we'll help you.",
      forgotBad:"Enter a valid email address.",
      resetHelp:"Choose a new password for your account.",
      resetNew:"New password",
      resetConfirm:"Repeat new password",
      resetSave:"Save password",
      resetDone:"Password changed. Sign in with your new password.",
      resetExpired:"This link has expired or was already used. Ask for a new one.",
      resetWeak:"Use at least 8 characters.",
      resetMismatch:"The two passwords don't match.",
      coEmail:"Email (optional, for your confirmation)",
      orderFailed:"Your order wasn't sent. Check your connection and try again, or order on WhatsApp.",
      privacyAgree:"Your details are used only to deliver your order.",
      privacyLink:"Privacy",
      privacyKicker:"Your data",
      privacyTitle:"Privacy notice",
      privacyIntro:"We collect only what we need to deliver your order, and we never sell it.",
      privacyCollectT:"What we keep",
      privacyCollect:"For an order: your name, phone, wilaya, commune and the items. Optionally your email, for the confirmation. With an account: your email and a protected (hashed) password. We also count page views and bag additions, without cookies and without names.",
      privacyUseT:"Who sees it",
      privacyUse:"The store team, and the delivery company you choose (name, phone and address only). Nothing is used for advertising.",
      privacyDeviceT:"On your device",
      privacyDevice:"Your bag, language and delivery details are saved in your browser so you don't retype them. A sign-in cookie keeps you logged in. There are no advertising or tracking cookies.",
      privacyRightsT:"Your rights",
      privacyRights:"Under Algerian law 18-07 you can ask to see, correct or delete your data. Message us on WhatsApp or through the Contact page.",
      title:"Noor — Thobe House | Men's Thobes, Djellabas & Shirts",
      bannerDefault:"Free delivery over 8,000 DZD · Cash on delivery in all 58 wilayas",
      navShop:"Thobes", navSize:"Size guide", navAbout:"Our house",
      h1e:"Noor · Thobe House — Algeria", h1t:"The thobe,<br><em>perfected.</em>",
      h1s:"Cut for Friday prayer, Eid and the family majlis. Breathable fabrics, honest finishing, sizes 52–58 — delivered to your door.",
      shopBest:"Shop the collection", findSize:"Find my size",
      h2e:"Collections", h2t:"Dress for <em>the occasion.</em>",
      h2s:"One house, three registers. Light cotton for everyday, deeper tones for Eid, and warm layers for the winter majlis.",
      col1t:"Everyday thobes", col1d:"Light cotton and crepe, cut for daily wear.", col2t:"Eid & evenings", col2d:"Deeper tones and satin touches for celebration.", col3t:"Djellabas", col3d:"Warm layers with sfifa trim and an easy drape.", colLink:"Shop →",
      h3e:"Fit, without guessing", h3t:"Find your <em>size.</em>",
      h3s:"Thobes are numbered by height, not chest. Pick your usual size; if you are between two, size up for a relaxed drape. Every order can be exchanged once within 7 days.",
      openGuide:"Open the size guide", sizeCol:"Size", heightCol:"Height", lengthCol:"Thobe length", chestCol:"Chest", sleeveCol:"Sleeve", sizeNote:"Measurements are garment measures. Between sizes? Choose the larger.",
      carE:"Bestsellers · drag or tap", carT:"The pieces <em>they reorder.</em>", shopAll:"Shop all pieces",
      shopTitle:"The collection", shopSub:"Thobes, djellabas and shirts — cut for prayer, Eid and everyday. Sizes 52–58, prices in DZD, cash on delivery.",
      searchPh:"Search thobes…", sortBy:"Sort", sortFeatured:"Featured", sortPriceAsc:"Price: low to high", sortPriceDesc:"Price: high to low", sortNew:"Newest",
      as1t:"Cash on delivery", as1d:"Pay the courier when your order arrives.", as2t:"Free delivery over 8,000 DZD", as2d:"Home or stopdesk, all 58 wilayas.", as3t:"7-day size exchange", as3d:"One exchange per order, no questions.",
      abTitle:"The Noor house", abSub:"Noor is a small-batch thobe house. Fewer, better pieces — honest fabrics, quiet details, no compromise.",
      ab1t:"Cut for real life", ab1d:"Every thobe is cut for easy movement and finished with double-stitched seams that survive daily wear and Friday crowds.",
      ab2t:"Fabric first", ab2d:"Breathable cotton, linen blends and soft crepe. No scratchy synthetics, no see-through surprises in daylight.",
      ab3t:"Small weekly drops", ab3d:"Released in small batches so colours stay fresh, stock stays honest, and every piece is checked before it ships.",
      abNoteT:"How sizing works", abNoteD:"Algerian thobes are numbered 52–58 by height. 52 fits roughly 165–170 cm, 58 fits 180–186 cm. If you are between sizes, choose the larger — the drape stays clean and the shoulders stay comfortable.",
      chooseSize:"Choose your size", sizeGuide:"Size guide", sizeHint:"Between sizes? Choose the larger for a relaxed drape.",
      fabric:"Fabric", fit:"Fit", care:"Care",
      expressKicker:"Express order", expressFast:"Ready in 30 seconds", orderNow:"Order now", exHome:"Home delivery", exStop:"Stopdesk",
      qvTotal:"Product total", qvTrustCod:"Cash on delivery", qvTrustShip:"2–5 day delivery", qvTrustExchange:"7-day exchange",
      expressSub:"Size and quantity are set. Confirm your details and we call you before shipping.",
      expressDetails:"Delivery details", expressPlace:"Confirm order · cash on delivery", expressWa:"Order on WhatsApp",
      expressSecure:"No online payment · We confirm by phone", expressSuccess:"Order received!",
      expressSuccessSub:"We will call you shortly to confirm the delivery.", expressContinue:"Keep shopping",
      expressWhatsApp:"Message us on WhatsApp", expressShipPending:"Select wilaya", expressSaved:"Details saved for next order", inStock:"In stock",
      expressFailed:"Order could not be placed. Try again or use WhatsApp.",
      d1t:"Cash on delivery", d1d:"Pay the courier when it arrives.", d2t:"2–5 day delivery", d2d:"All 58 wilayas · free over 8,000 DZD.", d3t:"7-day exchange", d3d:"One size exchange per order.",
      sizeGuideKicker:"Noor · fit guide", sizeGuideTitle:"Find your thobe size",
      sizeGuideIntro:"Algerian thobes are numbered by height. Measure yourself in centimetres, then choose the closest size. If you are between two sizes, size up.",
      how1t:"1 · Height", how1d:"Stand straight against a wall, without shoes. This is the main size guide.",
      how2t:"2 · Chest", how2d:"Measure around the fullest part, keeping the tape level and relaxed.",
      how3t:"3 · Length", how3d:"From the shoulder seam to the hem you prefer — ankle or floor.",
      sizeGuideNote:"Still unsure? Send your height and usual shirt size on WhatsApp — we will confirm before shipping.",
      cartNote:"Cash on delivery · Free delivery over 8,000 DZD",
      remove:"Remove", pickSize:"Choose your size first.", yourSize:"Size",
      contactNav:"Contact",
      contactKicker:"Noor · Contact & collaborations",
      contactTitle:"Let's talk.",
      contactIntro:"Orders, sizing, wholesale or a sponsored collaboration — reach the house directly.",
      contactWa:"WhatsApp the house", contactWaSub:"Fastest reply · orders & sizing",
      contactSocialK:"Follow Noor", contactSocialHint:"New drops, fabrics and behind the seams.",
      contactDetailsT:"The house", contactBasedL:"Based", contactBasedV:"Algiers, Algeria",
      contactMarketL:"Market", contactMarketV:"58 wilayas · DZD · cash on delivery",
      contactLangL:"Languages", contactLangV:"English · Français · العربية",
      sponsorKicker:"Sponsorships", sponsorT:"Create with Noor",
      sponsorD:"Paid collaborations, campaign features and event coverage for modest menswear.",
      sponsorFormats:"Reels · Stories · Lookbooks · Event features",
      contactNote:"For wholesale or sponsorship, message us with your idea and timeline.",
      contactCopy:"Copy media kit", contactCopied:"Media kit copied",
      contactWaMsg:"Noor: hello, I would like to ask about an order / collaboration.",
      foot:"Noor Thobe House &nbsp;·&nbsp; Algiers &nbsp;·&nbsp; Cash on delivery across Algeria"
    },
    fr: {
      forgotLink:"Mot de passe oublié ?",
      forgotHelp:"Saisissez l'e-mail de votre compte. Nous vous enverrons un lien pour choisir un nouveau mot de passe.",
      back:"Retour",
      forgotSend:"Envoyer le lien",
      forgotSent:"Si un compte existe pour cet e-mail, un lien de réinitialisation arrive. Vérifiez votre boîte de réception et vos spams.",
      forgotNoMail:"La réinitialisation par e-mail n'est pas encore disponible. Écrivez-nous sur WhatsApp, nous vous aiderons.",
      forgotBad:"Saisissez une adresse e-mail valide.",
      resetHelp:"Choisissez un nouveau mot de passe pour votre compte.",
      resetNew:"Nouveau mot de passe",
      resetConfirm:"Répétez le nouveau mot de passe",
      resetSave:"Enregistrer",
      resetDone:"Mot de passe modifié. Connectez-vous avec le nouveau mot de passe.",
      resetExpired:"Ce lien a expiré ou a déjà été utilisé. Demandez-en un nouveau.",
      resetWeak:"Utilisez au moins 8 caractères.",
      resetMismatch:"Les deux mots de passe ne correspondent pas.",
      coEmail:"E-mail (facultatif, pour la confirmation)",
      orderFailed:"Votre commande n'a pas été envoyée. Vérifiez votre connexion et réessayez, ou commandez sur WhatsApp.",
      privacyAgree:"Vos coordonnées servent uniquement à livrer votre commande.",
      privacyLink:"Confidentialité",
      privacyKicker:"Vos données",
      privacyTitle:"Confidentialité",
      privacyIntro:"Nous collectons uniquement ce qui est nécessaire pour livrer votre commande, et nous ne le vendons jamais.",
      privacyCollectT:"Ce que nous conservons",
      privacyCollect:"Pour une commande : nom, téléphone, wilaya, commune et articles. Votre e-mail si vous le donnez, pour la confirmation. Avec un compte : votre e-mail et un mot de passe protégé (haché). Nous comptons aussi les vues et les ajouts au panier, sans cookies et sans noms.",
      privacyUseT:"Qui les voit",
      privacyUse:"L'équipe de la boutique et la société de livraison choisie (nom, téléphone et adresse uniquement). Rien n'est utilisé pour la publicité.",
      privacyDeviceT:"Sur votre appareil",
      privacyDevice:"Votre panier, votre langue et vos coordonnées de livraison sont enregistrés dans votre navigateur pour éviter de les ressaisir. Un cookie de connexion vous garde connecté. Aucun cookie publicitaire ou de suivi.",
      privacyRightsT:"Vos droits",
      privacyRights:"Selon la loi algérienne 18-07, vous pouvez demander à consulter, corriger ou supprimer vos données. Écrivez-nous sur WhatsApp ou via la page Contact.",
      title:"Noor — Maison du Thobe | Thobes, Djellabas & Chemises",
      bannerDefault:"Livraison offerte dès 8 000 DZD · Paiement à la livraison dans les 58 wilayas",
      navShop:"Thobes", navSize:"Guide des tailles", navAbout:"La maison",
      h1e:"Noor · Maison du Thobe — Algérie", h1t:"Le thobe,<br><em>parfait.</em>",
      h1s:"Taillé pour la prière du vendredi, l'Aïd et les réunions de famille. Étoffes respirantes, finitions soignées, tailles 52–58 — livré chez vous.",
      shopBest:"Voir la collection", findSize:"Trouver ma taille",
      h2e:"Collections", h2t:"Habillé pour <em>l'occasion.</em>",
      h2s:"Une maison, trois registres. Coton léger pour tous les jours, teintes profondes pour l'Aïd, laines chaudes pour le majlis d'hiver.",
      col1t:"Thobes du quotidien", col1d:"Coton léger et crêpe, taillés pour tous les jours.", col2t:"Aïd & soirées", col2d:"Teintes profondes et touches satinées pour les fêtes.", col3t:"Djellabas", col3d:"Couches chaudes, sfifa et tombé facile.", colLink:"Voir →",
      h3e:"La taille, sans hésiter", h3t:"Trouvez votre <em>taille.</em>",
      h3s:"Les thobes se numérotent par hauteur, pas par poitrine. Prenez votre taille habituelle ; entre deux, choisissez la plus grande pour un tombé ample. Un échange possible sous 7 jours.",
      openGuide:"Ouvrir le guide des tailles", sizeCol:"Taille", heightCol:"Hauteur", lengthCol:"Longueur", chestCol:"Poitrine", sleeveCol:"Manche", sizeNote:"Mesures du vêtement. Entre deux tailles ? Prenez la plus grande.",
      carE:"Meilleures ventes · glissez ou touchez", carT:"Les pièces <em>qu'on rachète.</em>", shopAll:"Voir toutes les pièces",
      shopTitle:"La collection", shopSub:"Thobes, djellabas et chemises — pour la prière, l'Aïd et tous les jours. Tailles 52–58, prix en DZD, paiement à la livraison.",
      searchPh:"Rechercher un thobe…", sortBy:"Trier", sortFeatured:"Sélection", sortPriceAsc:"Prix croissant", sortPriceDesc:"Prix décroissant", sortNew:"Nouveautés",
      as1t:"Paiement à la livraison", as1d:"Payez le livreur à la réception.", as2t:"Livraison offerte dès 8 000 DZD", as2d:"Domicile ou stopdesk, 58 wilayas.", as3t:"Échange sous 7 jours", as3d:"Un échange par commande, sans discussion.",
      abTitle:"La maison Noor", abSub:"Noor est une maison de thobes en petites séries. Moins de pièces, meilleures — étoffes honnêtes, détails discrets, sans compromis.",
      ab1t:"Taillé pour la vraie vie", ab1d:"Chaque thobe est coupé pour bouger librement, avec des coutures doubles qui durent.",
      ab2t:"L'étoffe d'abord", ab2d:"Coton respirant, mélanges de lin et crêpe doux. Pas de synthétique qui gratte, pas de transparence.",
      ab3t:"Petites séries", ab3d:"Quelques pièces chaque semaine, contrôlées une à une avant l'envoi.",
      abNoteT:"Comment ça taille", abNoteD:"Les thobes algériens vont de 52 à 58 selon la hauteur. 52 correspond à 165–170 cm, 58 à 180–186 cm. Entre deux tailles, prenez la plus grande.",
      chooseSize:"Choisissez votre taille", sizeGuide:"Guide des tailles", sizeHint:"Entre deux tailles ? Prenez la plus grande.",
      fabric:"Étoffe", fit:"Coupe", care:"Entretien",
      expressKicker:"Commande express", expressFast:"Prêt en 30 secondes", orderNow:"Commander maintenant", exHome:"À domicile", exStop:"Au bureau",
      qvTotal:"Total produit", qvTrustCod:"Paiement à la livraison", qvTrustShip:"Livraison 2–5 jours", qvTrustExchange:"Échange 7 jours",
      expressSub:"Taille et quantité choisies. Confirmez vos coordonnées, nous vous appelons avant l'envoi.",
      expressDetails:"Détails de livraison", expressPlace:"Confirmer · paiement à la livraison", expressWa:"Commander sur WhatsApp",
      expressSecure:"Aucun paiement en ligne · Confirmation par téléphone", expressSuccess:"Commande reçue !",
      expressSuccessSub:"Nous vous appelons rapidement pour confirmer la livraison.", expressContinue:"Continuer mes achats",
      expressWhatsApp:"Écrire sur WhatsApp", expressShipPending:"Choisir la wilaya", expressSaved:"Coordonnées enregistrées pour la prochaine commande", inStock:"En stock",
      expressFailed:"La commande n'a pas pu être envoyée. Réessayez ou utilisez WhatsApp.",
      d1t:"Paiement à la livraison", d1d:"Payez le livreur à la réception.", d2t:"Livraison 2–5 jours", d2d:"58 wilayas · offerte dès 8 000 DZD.", d3t:"Échange 7 jours", d3d:"Un échange de taille par commande.",
      sizeGuideKicker:"Noor · guide des tailles", sizeGuideTitle:"Trouvez votre taille",
      sizeGuideIntro:"Les thobes algériens se numérotent par hauteur. Mesurez-vous en centimètres, puis choisissez la taille la plus proche. Entre deux, prenez la plus grande.",
      how1t:"1 · Hauteur", how1d:"Debout contre un mur, sans chaussures. C'est le repère principal.",
      how2t:"2 · Poitrine", how2d:"Mesurez au plus fort, ruban horizontal et détendu.",
      how3t:"3 · Longueur", how3d:"De l'épaule à l'ourlet souhaité — cheville ou sol.",
      sizeGuideNote:"Un doute ? Envoyez votre hauteur et votre taille de chemise sur WhatsApp — nous confirmons avant l'envoi.",
      cartNote:"Paiement à la livraison · Livraison offerte dès 8 000 DZD",
      remove:"Retirer", pickSize:"Choisissez d'abord votre taille.", yourSize:"Taille",
      contactNav:"Contact",
      contactKicker:"Noor · Contact & collaborations",
      contactTitle:"Parlons-en.",
      contactIntro:"Commandes, tailles, vente en gros ou collaboration sponsorisée — contactez la maison directement.",
      contactWa:"WhatsApp de la maison", contactWaSub:"Réponse la plus rapide · commandes & tailles",
      contactSocialK:"Suivre Noor", contactSocialHint:"Nouveautés, étoffes et coulisses.",
      contactDetailsT:"La maison", contactBasedL:"Basée à", contactBasedV:"Alger, Algérie",
      contactMarketL:"Marché", contactMarketV:"58 wilayas · DZD · paiement à la livraison",
      contactLangL:"Langues", contactLangV:"English · Français · العربية",
      sponsorKicker:"Sponsoring", sponsorT:"Créer avec Noor",
      sponsorD:"Collaborations rémunérées, campagnes et couverture d'événements pour la mode masculine modeste.",
      sponsorFormats:"Reels · Stories · Lookbooks · Événements",
      contactNote:"Pour la vente en gros ou un sponsoring, écrivez-nous avec votre idée et vos dates.",
      contactCopy:"Copier le kit média", contactCopied:"Kit média copié",
      contactWaMsg:"Noor : bonjour, je souhaite me renseigner sur une commande / collaboration.",
      foot:"Noor Maison du Thobe &nbsp;·&nbsp; Alger &nbsp;·&nbsp; Paiement à la livraison partout en Algérie"
    },
    ar: {
      forgotLink:"نسيت كلمة المرور؟",
      forgotHelp:"أدخل بريد حسابك الإلكتروني وسنرسل لك رابطاً لاختيار كلمة مرور جديدة.",
      back:"رجوع",
      forgotSend:"إرسال الرابط",
      forgotSent:"إذا كان هناك حساب بهذا البريد، فرابط إعادة التعيين في الطريق. تحقق من بريدك الوارد ومجلد الرسائل غير المرغوب فيها.",
      forgotNoMail:"إعادة تعيين كلمة المرور بالبريد غير متاحة بعد. راسلنا على واتساب وسنساعدك.",
      forgotBad:"أدخل بريداً إلكترونياً صحيحاً.",
      resetHelp:"اختر كلمة مرور جديدة لحسابك.",
      resetNew:"كلمة المرور الجديدة",
      resetConfirm:"أعد كتابة كلمة المرور الجديدة",
      resetSave:"حفظ كلمة المرور",
      resetDone:"تم تغيير كلمة المرور. سجّل الدخول بكلمة المرور الجديدة.",
      resetExpired:"انتهت صلاحية هذا الرابط أو استُخدم من قبل. اطلب رابطاً جديداً.",
      resetWeak:"استخدم 8 أحرف على الأقل.",
      resetMismatch:"كلمتا المرور غير متطابقتين.",
      coEmail:"البريد الإلكتروني (اختياري، لتأكيد الطلب)",
      orderFailed:"لم يتم إرسال طلبك. تحقق من اتصالك وحاول مجدداً، أو اطلب عبر واتساب.",
      privacyAgree:"نستخدم بياناتك فقط لتوصيل طلبك.",
      privacyLink:"الخصوصية",
      privacyKicker:"بياناتك",
      privacyTitle:"إشعار الخصوصية",
      privacyIntro:"نجمع فقط ما نحتاجه لتوصيل طلبك، ولا نبيعه أبداً.",
      privacyCollectT:"ما نحتفظ به",
      privacyCollect:"للطلب: الاسم والهاتف والولاية والبلدية والمنتجات. وبريدك الإلكتروني إن أدخلته لتأكيد الطلب. مع الحساب: بريدك وكلمة مرور محمية (مشفّرة). ونحصي أيضاً مشاهدات الصفحات والإضافات إلى السلة، دون ملفات تعريف الارتباط ودون أسماء.",
      privacyUseT:"من يطّلع عليها",
      privacyUse:"فريق المتجر وشركة التوصيل التي تختارها (الاسم والهاتف والعنوان فقط). لا يُستخدم أي شيء للإعلانات.",
      privacyDeviceT:"على جهازك",
      privacyDevice:"تُحفظ سلتك ولغتك وبيانات التوصيل في متصفحك حتى لا تعيد كتابتها. ملف تعريف ارتباط لتسجيل الدخول يُبقيك متصلاً. لا توجد ملفات تعريف ارتباط إعلانية أو للتتبع.",
      privacyRightsT:"حقوقك",
      privacyRights:"وفق القانون الجزائري 18-07 يمكنك طلب الاطلاع على بياناتك أو تصحيحها أو حذفها. راسلنا على واتساب أو عبر صفحة التواصل.",
      title:"نور — دار الثوب | ثوب وجلابيب وقمصان",
      bannerDefault:"توصيل مجاني فوق 8,000 دج · الدفع عند الاستلام في 58 ولاية",
      navShop:"الثياب", navSize:"دليل المقاسات", navAbout:"دار نور",
      h1e:"نور · دار الثوب — الجزائر", h1t:"الثوب،<br><em>بإتقان.</em>",
      h1s:"مفصّل لصلاة الجمعة والعيد ومجالس العائلة. أقمشة متنفسة، خياطة متقنة، مقاسات 52–58 — يوصلك إلى بابك.",
      shopBest:"تسوق التشكيلة", findSize:"اعرف مقاسك",
      h2e:"التشكيلات", h2t:"أناقة <em>لكل مناسبة.</em>",
      h2s:"دار واحدة وثلاثة أذواق. قطن خفيف لكل يوم، ألوان عميقة للعيد، ودفء لمجالس الشتاء.",
      col1t:"ثياب كل يوم", col1d:"قطن خفيف وكريب، مفصّل للاستعمال اليومي.", col2t:"العيد والمناسبات", col2d:"ألوان عميقة ولمسات ساتان للاحتفال.", col3t:"جلابيب", col3d:"دفء وسفيفة وانسدال سهل.", colLink:"تسوق ←",
      h3e:"المقاس بلا حيرة", h3t:"اعرف <em>مقاسك.</em>",
      h3s:"تُرقّم الثياب حسب الطول لا الصدر. اختر مقاسك المعتاد، وإن كنت بين مقاسين اختر الأكبر لانسدال مريح. يمكن تبديل واحد خلال 7 أيام.",
      openGuide:"افتح دليل المقاسات", sizeCol:"المقاس", heightCol:"الطول", lengthCol:"طول الثوب", chestCol:"الصدر", sleeveCol:"الكم", sizeNote:"القياسات خاصة بالثوب. بين مقاسين؟ اختر الأكبر.",
      carE:"الأكثر مبيعاً · اسحب أو المس", carT:"قطع <em>يعيدون شراءها.</em>", shopAll:"تسوق كل القطع",
      shopTitle:"التشكيلة", shopSub:"ثياب وجلابيب وقمصان — للصلاة والعيد وكل يوم. مقاسات 52–58، الأسعار بالدينار، والدفع عند الاستلام.",
      searchPh:"ابحث عن ثوب…", sortBy:"ترتيب", sortFeatured:"المختارة", sortPriceAsc:"السعر: الأقل أولاً", sortPriceDesc:"السعر: الأعلى أولاً", sortNew:"الأحدث",
      as1t:"الدفع عند الاستلام", as1d:"ادفع لعامل التوصيل عند وصول الطلب.", as2t:"توصيل مجاني فوق 8,000 دج", as2d:"للمنزل أو المكتب، 58 ولاية.", as3t:"تبديل خلال 7 أيام", as3d:"تبديل واحد لكل طلب، بلا أسئلة.",
      abTitle:"دار نور", abSub:"نور دار ثياب بدفعات صغيرة. قطع أقل وأفضل — أقمشة صادقة وتفاصيل هادئة بلا مساومة.",
      ab1t:"مفصّل للحياة", ab1d:"كل ثوب مفصّل لحركة مريحة ودرزات مزدوجة تصمد للاستعمال اليومي.",
      ab2t:"القماش أولاً", ab2d:"قطن متنفس وخلطات كتان وكريب ناعم. بلا ألياف خشنة ولا شفافية.",
      ab3t:"دفعات أسبوعية صغيرة", ab3d:"قطع محدودة كل أسبوع، تُفحص واحدة واحدة قبل الإرسال.",
      abNoteT:"كيف تعرف مقاسك", abNoteD:"الثياب الجزائرية تُرقّم من 52 إلى 58 حسب الطول. 52 يناسب 165–170 سم، و58 يناسب 180–186 سم. بين مقاسين، اختر الأكبر.",
      chooseSize:"اختر مقاسك", sizeGuide:"دليل المقاسات", sizeHint:"بين مقاسين؟ اختر الأكبر لانسدال مريح.",
      fabric:"القماش", fit:"القصّة", care:"العناية",
      expressKicker:"طلب سريع", expressFast:"جاهز في 30 ثانية", orderNow:"اطلب الآن", exHome:"إلى المنزل", exStop:"إلى المكتب",
      qvTotal:"مجموع المنتج", qvTrustCod:"الدفع عند الاستلام", qvTrustShip:"توصيل 2–5 أيام", qvTrustExchange:"تبديل 7 أيام",
      expressSub:"المقاس والكمية محددان. أكّد بياناتك وسنتصل بك قبل الإرسال.",
      expressDetails:"بيانات التوصيل", expressPlace:"تأكيد الطلب · الدفع عند الاستلام", expressWa:"اطلب عبر واتساب",
      expressSecure:"بلا دفع إلكتروني · نؤكد هاتفياً", expressSuccess:"تم استلام طلبك!",
      expressSuccessSub:"سنتصل بك قريباً لتأكيد التوصيل.", expressContinue:"مواصلة التسوق",
      expressWhatsApp:"راسلنا على واتساب", expressShipPending:"اختر الولاية", expressSaved:"تم حفظ بياناتك للطلب القادم", inStock:"متوفر",
      expressFailed:"تعذر إرسال الطلب. أعد المحاولة أو استخدم واتساب.",
      d1t:"الدفع عند الاستلام", d1d:"ادفع لعامل التوصيل عند الوصول.", d2t:"توصيل 2–5 أيام", d2d:"كل الولايات · مجاني فوق 8,000 دج.", d3t:"تبديل 7 أيام", d3d:"تبديل مقاس واحد لكل طلب.",
      sizeGuideKicker:"نور · دليل المقاسات", sizeGuideTitle:"اعرف مقاس ثوبك",
      sizeGuideIntro:"تُرقّم الثياب الجزائرية حسب الطول. قس نفسك بالسنتيمتر ثم اختر الأقرب. إن كنت بين مقاسين اختر الأكبر.",
      how1t:"1 · الطول", how1d:"قف مستقيماً بجانب الحائط دون حذاء. هذا هو المرجع الأساسي.",
      how2t:"2 · الصدر", how2d:"قس حول أوسع نقطة، مع إبقاء الشريط أفقياً ومرتاحاً.",
      how3t:"3 · الطول الكلي", how3d:"من كتف الثوب إلى الطول الذي تفضله — الكاحل أو الأرض.",
      sizeGuideNote:"ما زلت متردداً؟ أرسل طولك ومقاس قميصك على واتساب — نؤكد قبل الإرسال.",
      cartNote:"الدفع عند الاستلام · توصيل مجاني فوق 8,000 دج",
      remove:"إزالة", pickSize:"اختر مقاسك أولاً.", yourSize:"المقاس",
      contactNav:"اتصل بنا",
      contactKicker:"نور · تواصل ورعاية",
      contactTitle:"لنتحدث.",
      contactIntro:"الطلبات والمقاسات والبيع بالجملة أو تعاون برعاية — تواصل مع الدار مباشرة.",
      contactWa:"واتساب الدار", contactWaSub:"أسرع رد · الطلبات والمقاسات",
      contactSocialK:"تابع نور", contactSocialHint:"جديد التشكيلات والأقمشة وكواليس العمل.",
      contactDetailsT:"الدار", contactBasedL:"المقر", contactBasedV:"الجزائر العاصمة",
      contactMarketL:"السوق", contactMarketV:"58 ولاية · دينار جزائري · الدفع عند الاستلام",
      contactLangL:"اللغات", contactLangV:"English · Français · العربية",
      sponsorKicker:"الرعاية", sponsorT:"اصنع مع نور",
      sponsorD:"تعاون مدفوع وحملات وتغطية مناسبات للأزياء الرجالية المحتشمة.",
      sponsorFormats:"ريلز · ستوريز · لوك بوك · تغطية مناسبات",
      contactNote:"للبيع بالجملة أو الرعاية، راسلنا بفكرتك والموعد المناسب.",
      contactCopy:"نسخ ملف التعريف", contactCopied:"تم نسخ ملف التعريف",
      contactWaMsg:"نور: مرحباً، أود الاستفسار عن طلب / تعاون.",
      foot:"نور دار الثوب &nbsp;·&nbsp; الجزائر &nbsp;·&nbsp; الدفع عند الاستلام في كل الولايات"
    }
  };
  Object.keys(I18N_EXTRA).forEach(function(l){ if(I18N[l]) Object.assign(I18N[l], I18N_EXTRA[l]); });
  var lang = "en";
  try{ lang = storeGet("noor_lang", "en"); }catch(e){ lang = "en"; }
  if(!I18N[lang]) lang = "en";
  function t(k){ var d=I18N[lang]||I18N.en; var v=d[k]; if(v===undefined) v=I18N.en[k]; if(typeof v==="function") return v.apply(null, Array.prototype.slice.call(arguments,1)); return v; }
  var CATS = ["Thobes","Djellabas","Shirts","Accessories"];
  var CATL = { Thobes:{en:"Thobes",fr:"Thobes",ar:"ثياب"}, Djellabas:{en:"Djellabas",fr:"Djellabas",ar:"جلابيب"}, Shirts:{en:"Shirts",fr:"Chemises",ar:"قمصان"}, Accessories:{en:"Accessories",fr:"Accessoires",ar:"إكسسوارات"} };
  function catName(c){ return (CATL[c]&&CATL[c][lang])||c; }
  var WILAYAS = ["Adrar","Chlef","Laghouat","Oum El Bouaghi","Batna","Béjaïa","Biskra","Béchar","Blida","Bouira","Tamanrasset","Tébessa","Tlemcen","Tiaret","Tizi Ouzou","Alger","Djelfa","Jijel","Sétif","Saïda","Skikda","Sidi Bel Abbès","Annaba","Guelma","Constantine","Médéa","Mostaganem","M'Sila","Mascara","Ouargla","Oran","El Bayadh","Illizi","Bordj Bou Arréridj","Boumerdès","El Tarf","Tindouf","Tissemsilt","El Oued","Khenchela","Souk Ahras","Tipaza","Mila","Aïn Defla","Naâma","Aïn Témouchent","Ghardaïa","Relizane","Timimoun","Bordj Badji Mokhtar","Ouled Djellal","Béni Abbès","In Salah","In Guezzam","Touggourt","Djanet","El M'Ghair","El Meniaa"];
  var SHIPFEES = {6:[600,350],9:[500,300],13:[600,350],15:[600,350],16:[500,300],19:[500,300],23:[500,300],25:[500,300],30:[600,350],31:[500,300],35:[600,350],42:[600,350]};
  function shipFee(code, method){ var f=SHIPFEES[code]||[800,500]; return method==="stop"?f[1]:f[0]; }
  function applyLang(l){
    if(I18N[l]) lang=l;
    storeSet("noor_lang", lang);
    document.documentElement.lang = lang==="ar"?"ar":lang;
    document.documentElement.dir = lang==="ar"?"rtl":"ltr";
    document.title = t("title");
    Array.prototype.forEach.call(document.querySelectorAll(".langseg button"), function(b){ b.classList.toggle("on", b.getAttribute("data-lang")===lang); });
    Array.prototype.forEach.call(document.querySelectorAll("[data-i18n]"), function(el){ var v=t(el.getAttribute("data-i18n")); if(typeof v==="string" && el.innerHTML!==v) el.innerHTML=v; });
    Array.prototype.forEach.call(document.querySelectorAll("[data-i18n-ph]"), function(el){ var v=t(el.getAttribute("data-i18n-ph")); if(typeof v==="string") el.setAttribute("placeholder", v); });
    updateAuthUI(); renderChips(); renderProducts(); renderCart(); refreshOrders(); updateMiniAdds(); fillWilaya(); fillShipping(); coTotals();
    if(qvId){
      updateQvOrder();
      var qh=document.getElementById("qvSizeHint"); if(qh) qh.textContent = qvSize ? t("sizeHint") : t("pickSize");
    }
    if(expressModal && expressModal.classList.contains("open")){
      fillExpressWilaya(); fillExpressProviders(); renderExpressProduct(); renderExpressDone(); updateExpressWhatsApp();
    }
    if(SITE.banner) applyBanner(SITE.banner);
    else { var eb=document.getElementById("eventBanner"); if(eb) eb.textContent=t("bannerDefault"); }
  }
  var PRODUCTS = [
    { id:"thobe-grey-forest", name:"Thobe Adam — Grey · Forest Collar", size:"Sizes 52–58", desc:"Heather-grey thobe with bottle-green collar, cuffs and placket tab.", price:4900, min:4650, max:5150, tag:"New", cat:"Men", img:"images/thobe-grey-forest.jpg" , stock:12},
    { id:"thobe-taupe-silver", name:"Thobe Yassine — Taupe · Silver Trim", size:"Sizes 52–58", desc:"Stone-soft thobe with silver piping on collar, placket and cuffs.", price:5200, min:4950, max:5450, tag:"Bestseller", cat:"Men", img:"images/thobe-taupe-silver.jpg" , stock:12},
    { id:"thobe-brown-band", name:"Thobe Idris — Brown · Band Trim", size:"Sizes 52–58", desc:"Warm-brown thobe with woven band trim and gentle A-line drape.", price:4500, min:4300, max:4700, tag:"Classic", cat:"Men", img:"images/thobe-brown-band.jpg" , stock:12},
    { id:"thobe-navy-chain", name:"Thobe Fadil — Navy · Embroidered", size:"Sizes 52–58", desc:"Deep navy thobe with silver chain embroidery across placket and cuffs.", price:5600, min:5350, max:5850, tag:"Embroidered", cat:"Men", img:"images/thobe-navy-chain.jpg" , stock:12},
    { id:"thobe-navy-close", name:"Thobe Ezz — Navy · Close Fit", size:"Sizes 52–58", desc:"Sleek close-fit navy thobe in smooth charmeuse-touch fabric.", price:6900, min:6550, max:7250, tag:"Premium", cat:"Men", img:"images/thobe-navy-close.jpg" , stock:12},
    { id:"thobe-grey-hex", name:"Thobe Zaki — Grey · Hex Trim", size:"Sizes 52–58", desc:"Grey micro-check thobe with olive hexagon collar and cuff trims.", price:4700, min:4500, max:4900, tag:"New", cat:"Men", img:"images/thobe-grey-hex.jpg" , stock:12},
    { id:"thobe-beige-camel", name:"Thobe Bilal — Beige · Camel Collar", size:"Sizes 52–58", desc:"Sand-beige thobe with camel-brown tonal collar and cuffs.", price:4400, min:4200, max:4600, tag:"Everyday", cat:"Men", img:"images/thobe-beige-camel.jpg" , stock:12},
    { id:"thobe-black-gold", name:"Thobe Saif — Black · Gold Satin", size:"Sizes 52–58", desc:"Evening black satin-touch thobe with gold V-collar accents.", price:7900, min:7500, max:8300, tag:"Gold Accent", cat:"Men", img:"images/thobe-black-gold.jpg" , stock:12},
    { id:"djellaba-hood-grey", name:"Djellaba Waleed — Hooded Zip", size:"Free size", desc:"Winter djellaba with zip front, sfifa trim and side slits.", price:6400, min:6100, max:6700, tag:"Winter", cat:"Men", img:"images/djellaba-hood-grey.jpg" , stock:12},
    { id:"shirt-grey-collar", name:"Shirt Omar — Grey · Blue Contrast", size:"Sizes 52–58", desc:"Fold-to-go formal shirt, contrast blue collar and cuff buttons.", price:2400, min:2300, max:2500, tag:"Shirt", cat:"Men", img:"images/shirt-grey-collar.jpg" , stock:8},
    { id:"shirt-beige-brown", name:"Shirt Anis — Beige · Brown Contrast", size:"Sizes 52–58", desc:"Fold-to-go formal shirt, tan collar and gold square buttons.", price:2200, min:2100, max:2300, tag:"Shirt", cat:"Men", img:"images/shirt-beige-brown.jpg" , stock:8},
    { id:"thobe-khaliji", name:"ثوب الخليجي", size:"58", desc:"", price:9000, min:9000, max:9000, tag:"New", cat:"Thobes", img:"images/1000331028-ca129e6b371c.jpg", images:["images/1000331028-ca129e6b371c.jpg","images/1000331029-15946f1bf0ec.jpg","images/1000331030-47e4fa2ed5c6.jpg"], stock:1, sizes:["58"], fabric:"", care:"", fit:"", w:896, h:1200, imgset:[{w:320,url:"images/opt/1000331028-ca129e6b371c-320.webp"},{w:640,url:"images/opt/1000331028-ca129e6b371c-640.webp"},{w:896,url:"images/opt/1000331028-ca129e6b371c-896.webp"}]}
  ];
  var PRODUCT_META = {
    "thobe-grey-forest":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Cotton-blend crepe",care:"Machine wash 30° · hang dry",fit:"Regular — true to size"},
    "thobe-taupe-silver":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Stone-washed cotton",care:"Machine wash 30° · warm iron",fit:"Regular — true to size"},
    "thobe-brown-band":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Linen blend",care:"Machine wash 30° · hang dry",fit:"Relaxed A-line"},
    "thobe-navy-chain":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Embroidered poly-cotton",care:"Hand wash cold · do not tumble",fit:"Regular — true to size"},
    "thobe-navy-close":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Charmeuse-touch weave",care:"Hand wash cold · hang dry",fit:"Slim — size up if between"},
    "thobe-grey-hex":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Micro-check cotton",care:"Machine wash 30° · warm iron",fit:"Regular — true to size"},
    "thobe-beige-camel":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Cotton crepe",care:"Machine wash 30° · hang dry",fit:"Regular — true to size"},
    "thobe-black-gold":{cat:"Thobes",sizes:["52","54","56","58"],fabric:"Satin-touch polyester",care:"Hand wash cold · cool iron",fit:"Regular — true to size"},
    "djellaba-hood-grey":{cat:"Djellabas",sizes:["One size"],fabric:"Wool blend",care:"Dry clean recommended",fit:"Loose — one size"},
    "shirt-grey-collar":{cat:"Shirts",sizes:["52","54","56","58"],fabric:"Cotton poplin",care:"Machine wash 40° · warm iron",fit:"Regular — true to size"},
    "shirt-beige-brown":{cat:"Shirts",sizes:["52","54","56","58"],fabric:"Cotton poplin",care:"Machine wash 40° · warm iron",fit:"Regular — true to size"},
    "thobe-khaliji":{cat:"Thobes",sizes:["58"],size:"58",fabric:"",care:"",fit:""}
  };
  PRODUCTS = PRODUCTS.filter(function(p){ return !!PRODUCT_META[p.id]; }).map(function(p){
    var m = PRODUCT_META[p.id];
    p.cat = m.cat; p.sizes = m.sizes; p.fabric = m.fabric; p.care = m.care; p.fit = m.fit;
    p.size = m.size || (m.sizes.length === 1 ? "One size" : "Sizes 52–58");
    return p;
  });
  /* The fallback catalog is rendered before /api/products resolves. Attach
     the same local WebP variants the API returns, so a direct #shop visit
     does not fetch every JPEG and then replace it with a WebP. */
  var FALLBACK_VARIANT_WIDTHS = {
    "thobe-grey-forest":[320,640,896], "thobe-taupe-silver":[320,640,896],
    "thobe-brown-band":[320,640,896], "thobe-navy-chain":[320,640,896],
    "thobe-navy-close":[320,640,896], "thobe-grey-hex":[320,640,896],
    "thobe-beige-camel":[320,640,896], "thobe-black-gold":[320,640,896],
    "djellaba-hood-grey":[320,640,896], "shirt-grey-collar":[320,640],
    "shirt-beige-brown":[320,640]
  };
  PRODUCTS.forEach(function(p){
    var widths = FALLBACK_VARIANT_WIDTHS[p.id];
    if(!widths) return;
    p.imgset = widths.map(function(w){ return {w:w, url:"images/opt/"+p.id+"-"+w+".webp"}; });
  });
  var OCCASIONS = { "thobe-black-gold":"eid", "thobe-navy-chain":"eid", "thobe-navy-close":"eid", "djellaba-hood-grey":"winter" };
  var cart = storeGet("noor_cart", []);
  var grid = document.getElementById("productGrid");
  var cartItems = document.getElementById("cartItems");
  var cartTotal = document.getElementById("cartTotal");
  var cartCount = document.getElementById("cartCount");
  var cartDrawer = document.getElementById("cartDrawer");

  function money(n){ return Math.round(n).toLocaleString("en-US") + " DZD"; }
  function priceLabel(p){ return money(p.price); }
  /* Responsive product images: the backend attaches imgset only when optimized
     WebP files exist; the local catalog gets the same convention immediately. */
  function imgSetOf(p){
    /* The API sends imgset only for images that really have optimized WebP
       variants. Never guess a path here: uploaded product photos have no
       generated variants and a guessed URL would hide the card image. */
    return (p && Array.isArray(p.imgset)) ? p.imgset : [];
  }
  function imgAttrs(p, sizes, fallbackW){
    var set = imgSetOf(p), img = (p && p.img) || "";
    if(!set.length){ return ' src="'+escapeHtml(img)+'"'; }
    var chosen = set[0];
    set.forEach(function(v){ if(v.w <= fallbackW) chosen = v; });
    var srcset = set.map(function(v){ return v.url+" "+v.w+"w"; }).join(", ");
    return ' src="'+escapeHtml(chosen.url)+'" srcset="'+escapeHtml(srcset)+'" sizes="'+escapeHtml(sizes)+'"';
  }
  var IMG_PLACEHOLDER = "data:image/gif;base64,R0lGODlhAQABAAAAACw=";
  function lazyImgAttrs(p, sizes, fallbackW){
    var set = imgSetOf(p), img = (p && p.img) || "";
    if(!set.length){
      return ' src="'+IMG_PLACEHOLDER+'" data-src="'+escapeHtml(img)+'"';
    }
    var chosen = set[0];
    set.forEach(function(v){ if(v.w <= fallbackW) chosen = v; });
    var srcset = set.map(function(v){ return v.url+" "+v.w+"w"; }).join(", ");
    return ' src="'+IMG_PLACEHOLDER+'" data-src="'+escapeHtml(chosen.url)+'" data-srcset="'+escapeHtml(srcset)+'" data-sizes="'+escapeHtml(sizes)+'" sizes="'+escapeHtml(sizes)+'"';
  }
  function hydratePanelImgs(root){
    if(!root) return;
    var imgs = Array.prototype.slice.call(root.querySelectorAll("img[data-src]"));
    var i = 0;
    function next(){
      if(i >= imgs.length) return;
      var img = imgs[i++];
      if(!img.isConnected){ setTimeout(next, 0); return; }
      var src = img.getAttribute("data-src");
      var srcset = img.getAttribute("data-srcset");
      var sizes = img.getAttribute("data-sizes");
      if(srcset) img.srcset = srcset;
      if(sizes) img.sizes = sizes;
      if(src) img.src = src;
      img.removeAttribute("data-src");
      img.removeAttribute("data-srcset");
      img.removeAttribute("data-sizes");
      /* Set sources one at a time: six image loads in one task are a long
         task on a throttled phone. */
      setTimeout(next, 45);
    }
    next();
  }
  function applyImgSrc(img, p, sizes, fallbackW){
    var set = imgSetOf(p);
    if(!set.length){ img.removeAttribute("srcset"); img.removeAttribute("sizes"); img.src = (p && p.img) || ""; return; }
    var chosen = set[0];
    set.forEach(function(v){ if(v.w <= fallbackW) chosen = v; });
    img.src = chosen.url;
    img.srcset = set.map(function(v){ return v.url+" "+v.w+"w"; }).join(", ");
    img.sizes = sizes;
  }
  function storeGet(k,f){ try{ var v=localStorage.getItem(k); return v?JSON.parse(v):f; }catch(e){ return f; } }
  function storeSet(k,v){ try{ localStorage.setItem(k, JSON.stringify(v)); }catch(e){} }
  function saveCart(){ storeSet("noor_cart", cart); }
  var filterQ = "", filterCat = "All", filterOccasion = "", sortMode = "featured", shownN = 24;
  var lastGridSig = "";
  function gridSignature(list){
    return lang + "|" + list.map(function(p){
      var set=imgSetOf(p), first=set.length ? set[0].url : (p.img||"");
      return [p.id,p.name,p.price,p.tag,p.cat,p.size,first].join("\u001f");
    }).join("\u001e");
  }
  function stockInfo(p){
    var n = (p.stock===undefined ? 99 : (parseInt(p.stock,10)||0));
    return {n:n, out:n<=0, note:n<=0 ? t("oos") : (n<=5 ? t("lowLeft",n) : "")};
  }
  function cardMetaHtml(p){
    var st=stockInfo(p);
    return (p.size ? escapeHtml(p.size) : "") + (st.note ? ' <span class="stock-note '+(st.out?'out':'low')+'">· '+escapeHtml(st.note)+'</span>' : "");
  }
  function refreshCardStock(p){
    var card=null;
    Array.prototype.some.call(grid.children, function(el){
      if(el.getAttribute("data-pid")===p.id){ card=el; return true; }
      return false;
    });
    if(!card) return;
    var meta=card.querySelector(".card-meta");
    if(meta) meta.innerHTML=cardMetaHtml(p);
    var btn=card.querySelector(".add-btn"); if(!btn) return;
    var out=stockInfo(p).out;
    if(out && !btn.disabled){
      btn.outerHTML='<button class="pill add-btn" disabled style="opacity:.45;box-shadow:none">'+t("oos")+'</button>';
    } else if(!out && btn.disabled){
      btn.outerHTML='<button class="pill add-btn" data-choose="'+escapeHtml(p.id)+'">'+t("chooseSize")+'</button>';
    }
  }
  function renderProducts(){
    if(!shopView || !shopView.classList.contains("active")) return;
    var q = filterQ;
    var list = PRODUCTS.filter(function(p){
      if(filterCat!=="All" && p.cat!==filterCat) return false;
      if(filterOccasion && OCCASIONS[p.id]!==filterOccasion) return false;
      if(q && (p.name+" "+p.desc+" "+p.size+" "+p.fabric+" "+catName(p.cat)).toLowerCase().indexOf(q)<0) return false;
      return true;
    });
    if(sortMode==="price-asc") list = list.slice().sort(function(a,b){ return a.price-b.price; });
    else if(sortMode==="price-desc") list = list.slice().sort(function(a,b){ return b.price-a.price; });
    else if(sortMode==="new") list = list.slice().sort(function(a,b){ return (b.tag==="New"?1:0)-(a.tag==="New"?1:0); });
    var rc = document.getElementById("resultCount");
    if(rc) rc.textContent = Math.min(shownN, list.length)+" / "+list.length+" "+t("products");
    if(!list.length){ grid.innerHTML = '<div class="empty">'+t("noneFound")+'</div>'; lastGridSig=""; return; }
    var view = list.slice(0, shownN);
    var sig = gridSignature(view);
    /* /api/products normally resolves with the same catalog the fallback
       painted. Keep those cards (and their in-flight image requests) instead
       of rebuilding the grid and fetching every image a second time. */
    if(sig === lastGridSig && grid.children.length){ view.forEach(refreshCardStock); return; }
    lastGridSig = sig;
    var oldImgs = {};
    Array.prototype.forEach.call(grid.querySelectorAll(".card[data-pid] img.product-img"), function(im){
      var card=im.closest(".card"), key=card ? card.getAttribute("data-pid") : "";
      if(key && im.getAttribute("data-card-img")) oldImgs[key+"|"+im.getAttribute("data-card-img")] = im;
    });
    grid.innerHTML = "";
    view.forEach(function(p, idx){
      var d = document.createElement("div"); d.className="card"; d.setAttribute("data-pid", p.id);
      d.style.animationDelay = Math.min(idx*45, 600)+"ms";
      var dims = (p.w&&p.h)?' width="'+Number(p.w)+'" height="'+Number(p.h)+'"':' width="300" height="300"';
      var meta = cardMetaHtml(p);
      var imgLoad = idx===0 ? ' loading="eager" fetchpriority="high" decoding="async"' : ' loading="lazy" decoding="async"';
      var set=imgSetOf(p), first=set.length ? set[0].url : (p.img||"");
      d.innerHTML = '<div class="swatch"><img class="product-img" data-card-img="'+escapeHtml(first)+'"'+imgLoad+imgAttrs(p,"(max-width:720px) 46vw, 320px",320)+' alt="'+escapeHtml(p.name)+'"><span class="badge">'+escapeHtml(p.tag)+'</span></div><h3>'+escapeHtml(p.name)+'</h3><div class="size card-meta">'+meta+'</div><div class="price-row"><span class="price">'+escapeHtml(priceLabel(p))+'</span>'+(((p.stock===undefined)||p.stock>0)?'<button class="pill add-btn" data-choose="'+escapeHtml(p.id)+'">'+t("chooseSize")+'</button>':'<button class="pill add-btn" disabled style="opacity:.45;box-shadow:none">'+t("oos")+'</button>')+'</div>';
      var imgEl = d.querySelector("img.product-img");
      var oldImg = oldImgs[p.id+"|"+first];
      if(imgEl && oldImg){
        oldImg.className = imgEl.className;
        oldImg.alt = imgEl.alt;
        oldImg.width = imgEl.width; oldImg.height = imgEl.height;
        oldImg.loading = imgEl.loading;
        if(imgEl.getAttribute("fetchpriority")) oldImg.setAttribute("fetchpriority", imgEl.getAttribute("fetchpriority"));
        if(oldImg.getAttribute("src") !== imgEl.getAttribute("src")) oldImg.setAttribute("src", imgEl.getAttribute("src"));
        if(oldImg.getAttribute("srcset") !== imgEl.getAttribute("srcset")) oldImg.setAttribute("srcset", imgEl.getAttribute("srcset") || "");
        if(oldImg.getAttribute("sizes") !== imgEl.getAttribute("sizes")) oldImg.setAttribute("sizes", imgEl.getAttribute("sizes") || "");
        oldImg.removeAttribute("data-w");
        imgEl.replaceWith(oldImg);
        imgEl = oldImg;
      }
      if(imgEl) imgEl.addEventListener("error", function(){ this.style.display = "none"; }, {once:true});
      grid.appendChild(d);
    });
    watchImgs();
    var more = document.getElementById("loadMoreBtn");
    if(more){ more.hidden = shownN>=list.length; }
  }
  function renderChips(){
    var box = document.getElementById("catChips");
    if(!box || !shopView.classList.contains("active")) return;
    box.innerHTML = "";
    ["All"].concat(CATS).forEach(function(c){
      var b = document.createElement("button");
      b.type = "button"; b.className = "chip"+(filterCat===c?" on":"");
      b.textContent = c==="All"?t("all"):catName(c);
      b.addEventListener("click", function(){ filterCat=c; filterOccasion=""; shownN=12; renderChips(); renderProducts(); });
      box.appendChild(b);
    });
  }
  function lineKey(i){ return i.id+"::"+(i.size||""); }
  function findLine(id, size){ return cart.find(function(x){ return x.id===id && (x.size||"")===(size||""); }); }
  function renderCart(){
    var count = cart.reduce(function(s,i){return s+i.qty;},0);
    cartCount.textContent = count;
    if(!cart.length){ cartItems.innerHTML = '<p style="color:var(--fg-soft);font-size:14px">'+t("bagEmpty")+'</p>'; }
    else {
      cartItems.innerHTML = "";
      cart.forEach(function(i){
        var p = PRODUCTS.find(function(x){return x.id===i.id;}); if(!p) return;
        var row = document.createElement("div"); row.className="cart-item";
        var dims = (p.w&&p.h)?' width="'+Number(p.w)+'" height="'+Number(p.h)+'"':' width="56" height="56"';
        var thumb = p.img ? '<img class="cart-thumb" alt=""'+dims+imgAttrs(p,"96px",320)+'>' : '<div class="cart-thumb"></div>';
        var sizeTxt = i.size && i.size!=="One size" ? "Size "+i.size : (p.sizes&&p.sizes.length===1?p.sizes[0]:"");
        var key = escapeHtml(lineKey(i));
        row.innerHTML = thumb+'<div class="cart-meta"><b>'+escapeHtml(p.name)+'</b><small class="cart-size">'+escapeHtml(sizeTxt)+' · '+escapeHtml(money(p.price))+'</small><button class="cart-remove" data-remove="'+key+'" type="button">'+t("remove")+'</button></div><div class="qty"><button data-dec="'+key+'" aria-label="'+escapeHtml(t("dec"))+'">−</button><span>'+Number(i.qty)+'</span><button data-inc="'+key+'" aria-label="'+escapeHtml(t("inc"))+'">+</button></div>';
        cartItems.appendChild(row);
      });
    }
    var total = cart.reduce(function(s,i){ var p=PRODUCTS.find(function(x){return x.id===i.id;}); return s+(p?p.price*i.qty:0); },0);
    cartTotal.textContent = money(total);
  }
  function lineByKey(k){ return cart.find(function(x){ return lineKey(x)===k; }); }
  function productQty(id){ return cart.reduce(function(s,x){ return x.id===id?s+x.qty:s; },0); }
  function productById(id){ return PRODUCTS.find(function(x){ return x.id===id; }); }
  function sanitizeCart(){
    if(!cart.length) return;
    var next = [], changed = false;
    cart.forEach(function(it){
      var p = productById(it.id);
      if(!p || (p.stock !== undefined && p.stock <= 0)){ changed = true; return; }
      var sizes = p.sizes && p.sizes.length ? p.sizes : ["One size"];
      var size = it.size || (sizes.length === 1 ? sizes[0] : "");
      if(!size || sizes.indexOf(size) < 0){ changed = true; return; }
      var max = p.stock === undefined ? 99 : Math.max(0, Math.min(20, p.stock));
      if(max < 1){ changed = true; return; }
      var qty = Math.max(1, Math.min(Number(it.qty) || 1, max));
      if(qty !== it.qty || size !== it.size) changed = true;
      var found = next.find(function(x){ return x.id === it.id && x.size === size; });
      if(found){ found.qty = Math.min(max, found.qty + qty); changed = true; }
      else next.push({id:it.id, size:size, qty:qty});
    });
    if(changed || next.length !== cart.length){
      cart = next; saveCart(); renderCart();
    }
  }
  document.addEventListener("click", function(e){
    var choose = e.target.closest("[data-choose]");
    if(choose){ e.preventDefault(); openQV(choose.getAttribute("data-choose")); return; }
    var inc = e.target.closest("[data-inc]");
    if(inc){
      var itInc = lineByKey(inc.getAttribute("data-inc"));
      var pInc = itInc ? productById(itInc.id) : null;
      if(itInc){
        if(pInc && pInc.stock!==undefined && itInc.qty+1>pInc.stock){ notify(t("oos"),"error"); return; }
        itInc.qty++; saveCart(); renderCart();
      }
      return;
    }
    var dec = e.target.closest("[data-dec]");
    if(dec){
      var itDec = lineByKey(dec.getAttribute("data-dec"));
      if(itDec){ itDec.qty--; if(itDec.qty<=0) cart = cart.filter(function(x){ return x!==itDec; }); saveCart(); renderCart(); }
      return;
    }
    var rem = e.target.closest("[data-remove]");
    if(rem){
      var key = rem.getAttribute("data-remove");
      cart = cart.filter(function(x){ return lineKey(x)!==key; });
      saveCart(); renderCart(); notify(t("remove"),"info");
      return;
    }
    var card = e.target.closest("#productGrid .card");
    if(card && !e.target.closest("button")){ openQV(card.getAttribute("data-pid")); return; }
  });
  document.getElementById("cartOpen").addEventListener("click", function(){ cartDrawer.classList.add("open"); });
  document.getElementById("cartClose").addEventListener("click", function(){ cartDrawer.classList.remove("open"); });
  document.getElementById("checkoutBtn").addEventListener("click", function(){
    if(!cart.length){ notify(t("bagEmptyErr"),"error"); return; }
    cartDrawer.classList.remove("open");
    openCO();
  });
  function refreshOrders(){
    try{
      var orders = storeGet("noor_orders", []);
      if(!orders.length){ ordersInfo.textContent=t("ordersEmpty"); }
      else {
        ordersInfo.innerHTML=orders.slice(-5).reverse().map(function(o){
          return "<div>• "+(o.num?escapeHtml(o.num)+" — ":"")+Number(o.items||0)+" · "+escapeHtml(o.total||"")+"</div>";
        }).join("");
      }
      if(apiMode && apiToken && authState.email){
        api("/api/orders?email="+encodeURIComponent(authState.email), "GET", null, {"Authorization":"Bearer "+apiToken}).then(function(d){
          if(d && d.ok && d.orders && d.orders.length){
            ordersInfo.innerHTML=d.orders.slice(-5).reverse().map(function(o){
              return "<div>• "+(o.num?escapeHtml(o.num)+" — ":"")+Number(o.items||0)+" · "+escapeHtml(o.total||"")+"</div>";
            }).join("");
          }
        }).catch(function(){});
      }
    }catch(e){}
  }

  function notify(message,type){
    var t=document.createElement("div"); t.className="toast"+(type?" toast-"+type:""); t.textContent=message;
    toaster.appendChild(t);
    setTimeout(function(){ t.classList.add("out"); setTimeout(function(){ if(t.parentNode) t.parentNode.removeChild(t); },300); },3500);
  }
  var authState={ isLoggedIn: !!storeGet("noor_user", ""), email: storeGet("noor_user", "")||"" };
  function updateAuthUI(){
    var mBtn = document.getElementById("mobileAuthBtn");
    if(authState.isLoggedIn){
      authBtn.textContent = t("myNoor");
      if(mBtn) mBtn.textContent = t("myNoor");
      userGreeting.textContent = t("glowAcc", authState.email);
    } else {
      authBtn.innerHTML = t("navSignin");
      if(mBtn) mBtn.innerHTML = t("navSignin");
      userGreeting.textContent = t("clientWs");
    }
  }
  var isOverlayOpen=false;
  var adminCssLoaded = false;
  var shopPreloadUrl = "";
  function preloadShopImage(){
    var p = PRODUCTS[0]; if(!p) return;
    var set = imgSetOf(p); if(!set.length) return;
    var chosen = set[0];
    set.forEach(function(v){ if(v.w <= 640) chosen = v; });
    if(chosen.url === shopPreloadUrl) return;
    var old = document.getElementById("shopImagePreload");
    if(old) old.parentNode.removeChild(old);
    shopPreloadUrl = chosen.url;
    var link = document.createElement("link");
    link.id = "shopImagePreload";
    link.rel = "preload";
    link.as = "image";
    link.href = chosen.url;
    link.setAttribute("imagesrcset", set.map(function(v){ return v.url+" "+v.w+"w"; }).join(", "));
    link.setAttribute("imagesizes", "(max-width:720px) 46vw, 320px");
    link.setAttribute("fetchpriority", "high");
    document.head.appendChild(link);
  }
  function ensureAdminCss(){
    if(adminCssLoaded) return;
    adminCssLoaded = true;
    var link = document.createElement("link");
    link.id = "adminCss";
    link.rel = "stylesheet";
    link.href = "admin.css?v=20261027A";
    document.head.appendChild(link);
  }
  function handleRoute(){
    var hash=(window.location.hash.replace("#","")||"home");
    if(hash.indexOf("reset=")===0){
      // Password-reset link from the email: keep the token in memory only.
      resetToken = hash.slice(6).replace(/[^A-Za-z0-9_-]/g,"").slice(0,128);
      try{ history.replaceState(null, "", window.location.pathname + window.location.search + "#home"); }catch(e){}
      hash = "home";
      setTimeout(function(){ openAuthModal("reset"); var rp=document.getElementById("resetPass"); if(rp) rp.focus(); }, 60);
    }
    if(hash==="home"){ if(window.__carStart) window.__carStart(); }
    else if(window.__carStop) window.__carStop();
    document.body.classList.toggle("admin-mode", hash==="admin");
    if(hash!=="contact" && contactModal && contactModal.classList.contains("open")) closeContact();
    if(hash==="contact"){
      [shopView,aboutView,dashboardView,adminView].forEach(function(v){v.classList.remove("active");});
      document.querySelectorAll(".nav a").forEach(function(el){el.classList.remove("active");});
      document.querySelectorAll(".tabbar [data-tab]").forEach(function(el){el.classList.remove("on");});
      homePanels.style.opacity="1"; isOverlayOpen=false;
      var mmc=document.getElementById("mobileMenu"); if(mmc) mmc.classList.remove("open");
      openContact(); return;
    }
    if(hash.indexOf("/p/")===0){
      var deepId=hash.slice(3);
      [shopView,aboutView,dashboardView,adminView].forEach(function(v){v.classList.remove("active");});
      shopView.classList.add("active"); isOverlayOpen=true; homePanels.style.opacity="0";
      setTimeout(function(){ openQV(deepId); }, 80);
      return;
    }
    [shopView,aboutView,dashboardView,adminView].forEach(function(v){v.classList.remove("active");});
    document.querySelectorAll(".nav a").forEach(function(el){el.classList.remove("active");});
    document.querySelectorAll(".tabbar [data-tab]").forEach(function(el){el.classList.remove("on");});
    if(hash==="shop"){ preloadShopImage(); shopView.classList.add("active"); isOverlayOpen=true; homePanels.style.opacity="0"; renderChips(); renderProducts(); }
    else if(hash==="about"){ aboutView.classList.add("active"); isOverlayOpen=true; homePanels.style.opacity="0"; }
    else if(hash==="dashboard"){
      if(!authState.isLoggedIn){ window.location.hash="home"; openAuthModal(); return; }
      dashboardView.classList.add("active"); isOverlayOpen=true; homePanels.style.opacity="0";
    } else if(hash==="admin"){
      ensureAdminCss();
      cartDrawer.classList.remove("open");
      ["qvModal","expressModal","sizeModal","coModal","authModal"].forEach(function(id){
        var el=document.getElementById(id); if(el) el.classList.remove("open");
      });
      adminView.classList.add("active"); isOverlayOpen=true; homePanels.style.opacity="0"; renderAdmin();
      if(toaster) toaster.innerHTML="";
    } else { isOverlayOpen=false; homePanels.style.opacity="1"; readScroll(); }
    var n=document.querySelector('[data-route="'+hash+'"]'); if(n) n.classList.add("active");
    var t=document.querySelector('.tabbar [data-tab="'+hash+'"]'); if(t) t.classList.add("on");
    else if(hash==="home"){ var h=document.querySelector('.tabbar [data-tab="home"]'); if(h) h.classList.add("on"); }
    var mm=document.getElementById("mobileMenu"); if(mm) mm.classList.remove("open");
  }
  var forgotForm = document.getElementById("forgotForm"), resetForm = document.getElementById("resetForm"), resetToken = "";
  function authMode(m){
    authForm.hidden = m!=="login";
    if(forgotForm) forgotForm.hidden = m!=="forgot";
    if(resetForm) resetForm.hidden = m!=="reset";
  }
  function openAuthModal(mode){ authMode(mode||"login"); authModal.classList.add("open"); }
  function closeAuthModal(){ authModal.classList.remove("open"); authMode("login"); }
  function postJson(path, body){
    return fetch(path,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(body)}).then(function(r){
      return r.json().catch(function(){ return {}; }).then(function(j){ return {status:r.status, body:j||{}}; });
    });
  }
  document.getElementById("forgotBtn").addEventListener("click", function(){
    document.getElementById("forgotEmail").value = authEmail.value.trim();
    authMode("forgot"); document.getElementById("forgotEmail").focus();
  });
  document.getElementById("forgotBack").addEventListener("click", function(){ authMode("login"); });
  document.getElementById("resetCancel").addEventListener("click", function(){ resetToken = ""; closeAuthModal(); });
  forgotForm.addEventListener("submit", function(e){
    e.preventDefault();
    var email = document.getElementById("forgotEmail").value.trim().toLowerCase();
    var btn = document.getElementById("forgotSend"); btn.disabled = true;
    postJson("/api/auth/forgot",{email:email}).then(function(res){
      if(res.status===200 && res.body.ok){
        if(res.body.mail){ notify(t("forgotSent"),"success"); authEmail.value = email; authMode("login"); }
        else notify(t("forgotNoMail"),"info");
      } else if(res.status===429) notify(t("tooFast"),"error");
      else notify(t("forgotBad"),"error");
    }).catch(function(){ notify(t("orderFailed"),"error"); }).then(function(){ btn.disabled = false; });
  });
  resetForm.addEventListener("submit", function(e){
    e.preventDefault();
    var p1 = document.getElementById("resetPass").value, p2 = document.getElementById("resetPass2").value;
    if(p1.length < 8){ notify(t("resetWeak"),"error"); return; }
    if(p1 !== p2){ notify(t("resetMismatch"),"error"); return; }
    var btn = document.getElementById("resetSave"); btn.disabled = true;
    postJson("/api/auth/reset",{token:resetToken, pass:p1}).then(function(res){
      if(res.status===200 && res.body.ok){
        resetToken = ""; document.getElementById("resetPass").value = ""; document.getElementById("resetPass2").value = "";
        authEmail.value = res.body.email || ""; document.getElementById("authPass").value = "";
        notify(t("resetDone"),"success"); authMode("login"); document.getElementById("authPass").focus();
      } else if(res.status===429) notify(t("tooFast"),"error");
      else if(res.body.error==="weak") notify(t("resetWeak"),"error");
      else { notify(t("resetExpired"),"error"); authMode("forgot"); }
    }).catch(function(){ notify(t("orderFailed"),"error"); }).then(function(){ btn.disabled = false; });
  });
  authBtn.addEventListener("click", function(){ if(authState.isLoggedIn){ window.location.hash="dashboard"; } else openAuthModal(); });
  closeAuthBtn.addEventListener("click", closeAuthModal);
  authModal.addEventListener("click", function(e){ if(e.target===authModal) closeAuthModal(); });
  authForm.addEventListener("submit", function(e){
    e.preventDefault();
    var val=authEmail.value.trim().toLowerCase();
    var pass=document.getElementById("authPass").value;
    if(!val || pass.length<4){ notify(t("authErr"),"error"); return; }
    apiAuth(val, pass).then(function(ok){
      if(!ok) return;
      authState.isLoggedIn=true; authState.email=val; storeSet("noor_user",val);
      updateAuthUI(); closeAuthModal(); notify(t("hello", val),"success"); window.location.hash="dashboard";
      refreshOrders();
    });
  });
  logoutBtn.addEventListener("click", function(){
    if(apiToken) api("/api/auth/logout","POST",null,{"Authorization":"Bearer "+apiToken}).catch(function(){});
    authState.isLoggedIn=false; authState.email=""; try{ localStorage.removeItem("noor_user"); }catch(e){} apiToken=""; storeSet("noor_token","");
    updateAuthUI(); notify(t("bye"),"info"); window.location.hash="home";
  });
  window.addEventListener("hashchange", handleRoute);
  var privacyModal = document.getElementById("privacyModal");
  function openPrivacy(){ if(privacyModal) privacyModal.classList.add("open"); }
  function closePrivacy(){ if(privacyModal) privacyModal.classList.remove("open"); }
  Array.prototype.forEach.call(document.querySelectorAll("[data-open-privacy]"), function(b){ b.addEventListener("click", openPrivacy); });
  ["privacyClose","privacyClose2"].forEach(function(id){ var b=document.getElementById(id); if(b) b.addEventListener("click", closePrivacy); });
  if(privacyModal) privacyModal.addEventListener("click", function(e){ if(e.target===privacyModal) closePrivacy(); });
  document.addEventListener("keydown", function(e){ if(e.key==="Escape" && privacyModal && privacyModal.classList.contains("open")) closePrivacy(); });

  var progress=0, smoothProgress=0, docMax=0, lastProgress=-1;
  function readScroll(){
    if(isOverlayOpen) return;
    var y=window.pageYOffset||0;
    /* At the very top we know progress is zero; avoid forcing scrollHeight
       (a full document layout) before the first paint. */
    if(y<=0){ progress=0; schedulePaint(); return; }
    if(!docMax) docMax=Math.max(1, document.documentElement.scrollHeight-window.innerHeight);
    progress=clamp(y/docMax,0,1);
    schedulePaint();
  }
  function queuePanelWork(el){
    if(el._workQueued || el._worked) return;
    el._workQueued=true;
    var run=function(){
      el._workQueued=false;
      if(!el._cv || isOverlayOpen){ if(isOverlayOpen) el._cv=false; return; }
      el._worked=true;
      if(el.classList.contains("carousel-panel") && window.__carInit) window.__carInit();
      else hydratePanelImgs(el);
    };
    if(window.requestIdleCallback) requestIdleCallback(run,{timeout:1000});
    else setTimeout(run,150);
  }
  function paint(){
    if(Math.abs(progress-lastProgress)>0.0005){
      lastProgress=progress;
      meter.style.transform="scaleX("+progress+")";
    }
    if(isOverlayOpen) return;
    for(var i=0;i<panels.length;i++){
      var c=CUES[i]; if(!c) continue;
      var enter=ramp(progress,c[0],c[1]), leave=ramp(progress,c[2],c[3]);
      var o=enter*(1-leave), y=(1-enter)*DRIFT-leave*DRIFT, el=panels[i];
      var visible = o>0.005;
      if(!visible){
        if(el._cv!==false){
          el._cv=false;
          el.style.opacity="0";
          el.style.contentVisibility="hidden";
          el.style.visibility="hidden";
          el.style.pointerEvents="none";
          el.style.willChange="auto";
        }
        continue;
      }
      if(el._cv!==true){
        el._cv=true;
        el.style.contentVisibility="visible";
        el.style.visibility="visible";
        el.style.willChange="opacity, transform";
        queuePanelWork(el);
      }
      if(el._o!==o || el._y!==y){
        el._o=o; el._y=y;
        el.style.opacity=o.toFixed(3);
        el.style.transform="translate3d(0,"+y.toFixed(1)+"px,0)";
      }
      var pe=o>0.6?"auto":"none";
      if(el._pe!==pe){ el._pe=pe; el.style.pointerEvents=pe; }
    }
  }
  var paintRaf = 0;
  function schedulePaint(){
    if(paintRaf || document.hidden) return;
    paintRaf = requestAnimationFrame(function(){
      paintRaf = 0;
      if(!isOverlayOpen) scrubVideo();
      paint();
    });
  }
  /* ----- Lightweight dream background (theme-suited, near-zero weight) ----- */
  var parX = 0, parY = 0, parTX = 0, parTY = 0;
  function clipInit(){
    window.addEventListener("pointermove", function(e){
      if(isOverlayOpen) return;
      parTX = (e.clientX/window.innerWidth-0.5); parTY = (e.clientY/window.innerHeight-0.5);
      parX = parTX; parY = parTY; schedulePaint();
    }, {passive:true});
  }
  function scrubVideo(){
    smoothProgress = progress;
    if(dream){
      dream.style.transform = "translate3d("+(parX*-26).toFixed(2)+"px,"+((parY*-18)-(smoothProgress*70)).toFixed(2)+"px,0) scale("+(1.02+smoothProgress*0.05).toFixed(4)+")";
    }
  }
  document.addEventListener("visibilitychange", function(){ if(!document.hidden) schedulePaint(); });
  var scrollTick=0;
  function onScroll(){
    if(scrollTick) return;
    scrollTick=requestAnimationFrame(function(){ scrollTick=0; readScroll(); });
  }
  window.addEventListener("scroll",onScroll,{passive:true});
  var resizeTick=0;
  function onResize(){
    if(resizeTick) return;
    resizeTick=requestAnimationFrame(function(){
      resizeTick=0; docMax=0;
      if(window.__carResize) window.__carResize();
      readScroll(); paint();
    });
  }
  window.addEventListener("resize",onResize,{passive:true});
  /* ----- Mobile menu + tabbar ----- */
  (function(){
    var burger=document.getElementById("burgerBtn"), mm=document.getElementById("mobileMenu");
    if(burger&&mm){ burger.addEventListener("click", function(){ mm.classList.toggle("open"); }); }
    var mAuth=document.getElementById("mobileAuthBtn");
    if(mAuth){ mAuth.addEventListener("click", function(){ if(mm) mm.classList.remove("open"); if(authState.isLoggedIn){ window.location.hash="dashboard"; } else openAuthModal(); }); }
    var tabBag=document.getElementById("tabBag");
    if(tabBag){ tabBag.addEventListener("click", function(){ cartDrawer.classList.add("open"); }); }
  })();
  /* ----- Contact + sponsorship popup ----- */
  (function(){
    function openFromMenu(){
      var mm=document.getElementById("mobileMenu"); if(mm) mm.classList.remove("open");
      openContact();
    }
    ["navContact","footContact","mContact"].forEach(function(id){
      var el=document.getElementById(id);
      if(!el) return;
      el.addEventListener("click", function(e){ e.preventDefault(); openFromMenu(); });
    });
    var closeA=document.getElementById("contactClose"), closeB=document.getElementById("contactClose2");
    if(closeA) closeA.addEventListener("click", closeContact);
    if(closeB) closeB.addEventListener("click", closeContact);
    if(contactModal){
      contactModal.addEventListener("click", function(e){ if(e.target===contactModal) closeContact(); });
    }
    var copy=document.getElementById("contactCopy");
    if(copy) copy.addEventListener("click", copyContactKit);
  })();
  /* ----- Artistic 3D product carousel ----- */
  (function(){
    var track=document.getElementById("carTrack"), dots=document.getElementById("carDots");
    var prev=document.getElementById("carPrev"), next=document.getElementById("carNext"), zone=document.getElementById("car3d");
    if(!track) return;
    var ids=["thobe-black-gold","thobe-grey-forest","thobe-beige-camel","thobe-navy-chain","djellaba-hood-grey","thobe-taupe-silver"];
    var items=[], cur=0, timer=null, inited=false;
    var auto = !reduceMotion && window.innerWidth > 900;
    function layout(){
      if(!items.length) return;
      var isMobile=window.innerWidth<=720;
      var spread=isMobile?150:250, depth=isMobile?260:420, tilt=isMobile?28:42;
      items.forEach(function(p,i){
        var el=track.children[i]; if(!el) return;
        var off=i-cur;
        if(off>items.length/2) off-=items.length;
        if(off<-items.length/2) off+=items.length;
        var ax=Math.abs(off);
        var x=off*spread, z=-ax*(depth/items.length)- (ax>2?120:0), ry=off===0?0:(off>0?-tilt:tilt);
        var op=ax>2?0:(1-ax*0.22), sc=off===0?1:Math.max(0.72,1-ax*0.12);
        el.style.transform="translateX("+x+"px) translateZ("+z+"px) rotateY("+ry+"deg) scale("+sc+")";
        el.style.opacity=op; el.style.zIndex=100-ax;
        el.style.pointerEvents=Math.abs(off)>2?"none":"auto";
      });
      if(dots){ Array.prototype.forEach.call(dots.children, function(d,i){ d.classList.toggle("on", i===cur); }); }
    }
    function go(n){ if(!items.length) return; cur=(n+items.length)%items.length; layout(); restart(); }
    function restart(){ if(timer) clearInterval(timer); if(!auto || !inited) return; timer=setInterval(function(){ if(!document.hidden && !isOverlayOpen){ cur=(cur+1)%items.length; layout(); } }, 5200); }
    function init(){
      if(inited) return;
      inited=true;
      items=ids.map(function(id){ return PRODUCTS.find(function(p){return p.id===id;}); }).filter(Boolean);
      var built = 0;
      function addCard(){
        if(built >= items.length){ layout(); restart(); hydratePanelImgs(track); return; }
        var p=items[built], at=built;
        var c=document.createElement("div"); c.className="car-card";
        c.innerHTML='<img loading="lazy" decoding="async"'+(p.w&&p.h?' width="'+Number(p.w)+'" height="'+Number(p.h)+'"':' width="300" height="300"')+lazyImgAttrs(p,"230px",640)+' alt="'+escapeHtml(p.name)+'"><b>'+escapeHtml(p.name)+'</b><small>'+escapeHtml(p.size)+' · '+escapeHtml(priceLabel(p))+'</small><button class="pill mini-add" data-choose="'+escapeHtml(p.id)+'">'+t("chooseSize")+'</button>';
        c.addEventListener("click", function(e){ if(e.target.closest("[data-choose]")) return; go(at); });
        track.appendChild(c);
        if(dots){ var d=document.createElement("button"); d.type="button"; d.setAttribute("aria-label","Go to "+(at+1)); d.addEventListener("click", function(){ go(at); }); dots.appendChild(d); }
        built++;
        setTimeout(addCard, 0);
      }
      addCard();
    }
    window.__carInit = init;
    window.__carStop = function(){ if(timer){ clearInterval(timer); timer=null; } };
    window.__carStart = restart;
    window.__carResize = layout;
    if(prev) prev.addEventListener("click", function(){ init(); go(cur-1); });
    if(next) next.addEventListener("click", function(){ init(); go(cur+1); });
    if(zone){
      var sx=0, dx=0, touching=false;
      zone.addEventListener("touchstart", function(e){ touching=true; sx=e.touches[0].clientX; if(timer) clearInterval(timer); }, {passive:true});
      zone.addEventListener("touchmove", function(e){ if(!touching) return; dx=e.touches[0].clientX-sx; }, {passive:true});
      zone.addEventListener("touchend", function(){ init(); touching=false; if(Math.abs(dx)>40 && items.length){ cur=(cur+(dx<0?1:-1)+items.length)%items.length; layout(); } dx=0; restart(); });
      zone.addEventListener("mouseenter", function(){ if(timer) clearInterval(timer); });
      zone.addEventListener("mouseleave", restart);
    }
    document.addEventListener("visibilitychange", function(){
      if(document.hidden){ if(timer){ clearInterval(timer); timer=null; } }
      else if(!timer){ restart(); }
    });
  })();
  /* ----- Shop search ----- */
  var shopSearch = document.getElementById("shopSearch"), searchTimer=0;
  if(shopSearch){ shopSearch.addEventListener("input", function(){ clearTimeout(searchTimer); searchTimer=setTimeout(function(){ filterQ = shopSearch.value.trim().toLowerCase(); shownN = 12; renderProducts(); }, 140); }); }
  var loadMoreBtn = document.getElementById("loadMoreBtn");
  if(loadMoreBtn){ loadMoreBtn.addEventListener("click", function(){ shownN += 12; renderProducts(); }); }
  var shopSort = document.getElementById("shopSort");
  if(shopSort){ shopSort.addEventListener("change", function(){ sortMode = shopSort.value; shownN = 12; renderProducts(); }); }
  Array.prototype.forEach.call(document.querySelectorAll(".collection-card"), function(card){
    card.addEventListener("click", function(){
      filterCat = card.getAttribute("data-collection") || "All";
      var tag = card.getAttribute("data-tag");
      filterOccasion = tag ? tag.toLowerCase() : "";
      clearTimeout(searchTimer);
      filterQ = ""; if(shopSearch) shopSearch.value = ""; shownN = 12;
      renderChips(); renderProducts();
    });
  });
  /* ----- Size guide ----- */
  var sizeModal = document.getElementById("sizeModal");
  function openSizeGuide(){
    var mm = document.getElementById("mobileMenu"); if(mm) mm.classList.remove("open");
    if(sizeModal) sizeModal.classList.add("open");
  }
  function closeSizeGuide(){ if(sizeModal) sizeModal.classList.remove("open"); }
  ["navSizeGuide","heroSizeGuide","sizeGuideBtn2","sizeGuideBtn3","mSizeGuide","qvSizeGuide"].forEach(function(id){
    var b = document.getElementById(id);
    if(b) b.addEventListener("click", function(e){ e.preventDefault(); openSizeGuide(); });
  });
  ["sizeClose","sizeGuideClose"].forEach(function(id){
    var b = document.getElementById(id);
    if(b) b.addEventListener("click", closeSizeGuide);
  });
  if(sizeModal) sizeModal.addEventListener("click", function(e){ if(e.target===sizeModal) closeSizeGuide(); });
  /* ----- Quick view ----- */
  var qvModal = document.getElementById("qvModal");
  var qvId = null, qvN = 1, qvSize = "";
  var expressModal = document.getElementById("expressModal");
  var exId = null, exQty = 1, exSize = "", exInflight = false, exDoneData = null;
  function qvSizesOf(p){ return (p.sizes&&p.sizes.length)?p.sizes:["One size"]; }
  function renderQvSizes(p){
    var box = document.getElementById("qvSizes");
    if(!box) return;
    box.innerHTML = "";
    qvSizesOf(p).forEach(function(s){
      var b = document.createElement("button");
      b.type = "button"; b.className = "size-btn"+(qvSize===s?" on":"");
      b.textContent = s; b.setAttribute("data-qvsize", s);
      b.addEventListener("click", function(){
        qvSize = s;
        Array.prototype.forEach.call(box.children, function(x){ x.classList.toggle("on", x.getAttribute("data-qvsize")===s); });
        document.getElementById("qvSizeHint").textContent = t("sizeHint");
      });
      box.appendChild(b);
    });
    var hint = document.getElementById("qvSizeHint");
    if(hint) hint.textContent = qvSize ? t("sizeHint") : t("pickSize");
  }
  function qvMax(){
    var p = productById(qvId);
    if(!p || p.stock===undefined) return 99;
    return Math.max(1, parseInt(p.stock,10)||0);
  }
  function updateQvOrder(){
    var p = productById(qvId); if(!p) return;
    qvN = Math.max(1, Math.min(qvN, qvMax()));
    var qty = document.getElementById("qvQty"); if(qty) qty.textContent = qvN;
    var total = document.getElementById("qvTotal"); if(total) total.textContent = money(p.price*qvN);
    var stock = document.getElementById("qvStock");
    if(stock){
      var s = (p.stock===undefined?99:parseInt(p.stock,10)||0);
      stock.textContent = s<=0 ? t("oos") : (s<=5 ? t("lowLeft", s) : t("inStock"));
      stock.classList.toggle("out", s<=0);
      stock.classList.toggle("low", s>0 && s<=5);
    }
    var oos = (p.stock!==undefined && (parseInt(p.stock,10)||0)<=0);
    var add = document.getElementById("qvAdd"); if(add) add.disabled = oos;
    var buy = document.getElementById("qvExpress"); if(buy) buy.disabled = oos;
  }
  function customerDetails(){
    var c = storeGet("noor_customer", null);
    return (c && typeof c==="object") ? c : null;
  }
  function saveCustomerDetails(payload){
    storeSet("noor_customer", {
      name:payload.name||"", phone:payload.phone||"", wilaya:String(payload.wilaya||""),
      commune:payload.commune||"", method:payload.method||"home", provider:payload.provider||"standard"
    });
  }
  function fillExpressWilaya(keep){
    var sel = document.getElementById("exWilaya"); if(!sel) return;
    var val = (keep===undefined || keep===null) ? sel.value : String(keep);
    sel.innerHTML = "";
    var o0 = document.createElement("option"); o0.value = ""; o0.textContent = "— "+t("coWilaya")+" —";
    sel.appendChild(o0);
    WILAYAS.forEach(function(w, i){
      var o = document.createElement("option"); o.value = (i+1); o.textContent = (i+1)+" — "+w;
      sel.appendChild(o);
    });
    if(val) sel.value = String(val);
  }
  function fillExpressProviders(keep){
    var s = document.getElementById("exProvider"); if(!s) return;
    var provs = (SHIPDATA&&SHIPDATA.providers) ? SHIPDATA.providers : {standard:{name:"Noor Standard"}};
    var keys = Object.keys(provs);
    var val = (keep===undefined || keep===null) ? (s.value||"standard") : String(keep);
    s.innerHTML = "";
    keys.forEach(function(k){
      var o = document.createElement("option"); o.value = k; o.textContent = provs[k].name||k; s.appendChild(o);
    });
    s.value = provs[val] ? val : (provs.standard ? "standard" : (keys[0]||"standard"));
    var field = document.getElementById("exProviderField");
    if(field) field.hidden = keys.length < 2;
  }
  function exMethod(){
    var r = document.querySelector('input[name="exshipm"]:checked');
    return r ? r.value : "home";
  }
  function exMath(){
    var p = productById(exId);
    if(!p) return {sub:0,ship:0,total:0,n:0,code:0,method:"home",prov:"standard",pending:true};
    var sub = p.price*exQty;
    var wsel = document.getElementById("exWilaya");
    var code = parseInt((wsel&&wsel.value)||"0",10);
    var psel = document.getElementById("exProvider");
    var prov = (psel&&psel.value)||"standard";
    var method = exMethod();
    var freeOver = (SHIPDATA&&SHIPDATA.freeOver)||8000;
    var ship = 0;
    if(code) ship = ((shipFree(prov)&&sub>=freeOver)||sub===0) ? 0 : shipFee2(prov, code, method);
    return {sub:sub, ship:ship, total:sub+ship, n:exQty, code:code, method:method, prov:prov, pending:!code};
  }
  function updateExpressTotals(){
    if(!exId) return;
    var m = exMath();
    var sub = document.getElementById("exSubV"); if(sub) sub.textContent = money(m.sub);
    var ship = document.getElementById("exShipV"); if(ship) ship.textContent = m.pending ? t("expressShipPending") : (m.ship===0 ? t("free") : money(m.ship));
    var total = document.getElementById("exTotalV"); if(total) total.textContent = money(m.total);
    var unit = document.getElementById("exUnit"); if(unit) unit.textContent = money(m.sub);
    var qty = document.getElementById("exQty"); if(qty) qty.textContent = exQty;
  }
  function renderExpressProduct(){
    var p = productById(exId); if(!p) return;
    var im = document.getElementById("exProdImg");
    if(im){ applyImgSrc(im, p, "72px", 160); im.alt = p.name; }
    var nm = document.getElementById("exProdName"); if(nm) nm.textContent = p.name;
    var meta = document.getElementById("exProdMeta"); if(meta) meta.textContent = catName(p.cat)+" · "+t("yourSize")+" "+exSize;
    updateExpressTotals();
  }
  function updateExpressWhatsApp(){
    var show = !!WHATSAPP_NUMBER;
    var wa = document.getElementById("exWa"); if(wa) wa.hidden = !show;
    var dw = document.getElementById("exDoneWa"); if(dw) dw.hidden = !show;
  }
  function openExpress(){
    if(!qvId) return;
    var p = productById(qvId); if(!p) return;
    if(!qvSize){
      notify(t("pickSize"),"error");
      var sb = document.getElementById("qvSizes"); if(sb) sb.scrollIntoView({behavior: reduceMotion ? "auto" : "smooth", block:"center"});
      return;
    }
    if(p.stock!==undefined && (parseInt(p.stock,10)||0)<=0){ notify(t("oos"),"error"); return; }
    exId = qvId; exSize = qvSize;
    exQty = Math.max(1, Math.min(qvN, p.stock!==undefined ? (parseInt(p.stock,10)||1) : 99));
    exInflight = false; exDoneData = null;
    var form = document.getElementById("exForm"), done = document.getElementById("exDone");
    if(form) form.hidden = false; if(done) done.hidden = true;
    ["exEName","exEPhone","exEWilaya","exECommune"].forEach(function(id){ var e=document.getElementById(id); if(e) e.hidden=true; });
    var place = document.getElementById("exPlace"); if(place){ place.disabled=false; place.textContent=t("expressPlace"); }
    var c = customerDetails()||{};
    var nameEl=document.getElementById("exNameIn"), phoneEl=document.getElementById("exPhoneIn"), comEl=document.getElementById("exCommuneIn");
    if(nameEl) nameEl.value = c.name||"";
    if(phoneEl) phoneEl.value = c.phone||"";
    if(comEl) comEl.value = c.commune||"";
    fillExpressWilaya(c.wilaya||"");
    fillExpressProviders(c.provider||"standard");
    var method = c.method==="stop" ? "stop" : "home";
    Array.prototype.forEach.call(document.querySelectorAll('input[name="exshipm"]'), function(r){ r.checked = (r.value===method); });
    renderExpressProduct();
    updateExpressWhatsApp();
    /* One modal at a time: nested Chromium layers can ghost the quick-view
       text through the express sheet. The express card carries the product. */
    qvModal.classList.remove("open");
    expressModal.classList.add("open");
    track("begin_checkout", {source:"express", id:exId, size:exSize});
    setTimeout(function(){
      var f = (nameEl && !nameEl.value) ? nameEl : (document.getElementById("exWilaya")||nameEl);
      if(f) try{ f.focus(); }catch(e){}
    }, 120);
  }
  function closeExpress(){
    if(!expressModal) return;
    expressModal.classList.remove("open");
    if(window.location.hash.indexOf("#/p/")===0){
      try{ history.replaceState(null, "", window.location.pathname + window.location.search + "#shop"); }
      catch(e){ window.location.hash = "shop"; }
    }
  }
  function expressWaLink(){
    if(!WHATSAPP_NUMBER) return "";
    var p = productById(exId); if(!p) return "";
    var nameEl=document.getElementById("exNameIn"), phoneEl=document.getElementById("exPhoneIn"), comEl=document.getElementById("exCommuneIn");
    var name=nameEl?nameEl.value.trim():"", phone=phoneEl?phoneEl.value.replace(/\D/g,""):"", comm=comEl?comEl.value.trim():"";
    var m = exMath();
    var lines = ["Noor — "+t("expressKicker"),
      "• "+p.name+" × "+exQty+(exSize?" · "+t("yourSize")+" "+exSize:"")+" — "+money(p.price*exQty)];
    if(name) lines.push(t("coName")+": "+name);
    if(phone) lines.push(t("coPhone")+": "+phone);
    if(m.code) lines.push(t("coWilaya")+": "+WILAYAS[m.code-1]);
    if(comm) lines.push(t("coCommune")+": "+comm);
    lines.push(t("coShip")+": "+(m.method==="stop"?t("coStop"):t("coHome")));
    lines.push(t("coTotal")+": "+money(m.total));
    return "https://wa.me/"+WHATSAPP_NUMBER+"?text="+encodeURIComponent(lines.join("\n"));
  }
  function renderExpressDone(){
    if(!exDoneData) return;
    var el = document.getElementById("exDoneInfo");
    if(el) el.innerHTML = orderInfoHtml(exDoneData.num, exDoneData.n, exDoneData.total);
  }
  function finishExpress(num, n, total, payload){
    if(payload) saveCustomerDetails(payload);
    exDoneData = {num:num, n:n, total:total};
    var form=document.getElementById("exForm"), done=document.getElementById("exDone");
    if(form) form.hidden=true; if(done) done.hidden=false;
    renderExpressDone();
    refreshOrders();
    syncStock().then(function(){ renderProducts(); }).catch(function(){});
    updateExpressWhatsApp();
    notify(t("coPlaced", n, total), "success");
    track("order_placed", {n:n, source:"express"});
    var box = expressModal.querySelector(".ex-box"); if(box) box.scrollTop = 0;
  }
  function productImages(p){
    var out = [], seen = {};
    function add(url){
      url = String(url || "");
      if(!url || seen[url]) return;
      seen[url] = 1; out.push(url);
    }
    if(p){ add(p.img); if(Array.isArray(p.images)) p.images.forEach(add); }
    return out;
  }
  function renderQvThumbs(p){
    var box = document.getElementById("qvThumbs");
    if(!box) return;
    var imgs = productImages(p);
    box.innerHTML = "";
    if(imgs.length < 2){ box.hidden = true; return; }
    imgs.forEach(function(url, i){
      var b = document.createElement("button");
      b.type = "button"; b.className = "qv-thumb" + (i===0 ? " on" : "");
      b.setAttribute("data-qvimg", url);
      b.setAttribute("aria-label", "View image " + (i+1));
      var thumb = document.createElement("img");
      thumb.src = url; thumb.alt = ""; thumb.loading = "lazy";
      thumb.width = 64; thumb.height = 64;
      b.appendChild(thumb); box.appendChild(b);
    });
    box.hidden = false;
  }
  function fitQvMedia(im){
    var media = document.querySelector("#qvModal .qv-media");
    if(!media || !im || !im.naturalWidth || !im.naturalHeight) return;
    if(!window.matchMedia || !window.matchMedia("(max-width:720px)").matches){
      media.style.aspectRatio = "";
      return;
    }
    /* Give the phone hero the photo's own shape: portrait photos fill the
       sheet, square detail shots do not get letterboxed. */
    var ratio = Math.max(0.6, Math.min(1.6, im.naturalWidth / im.naturalHeight));
    media.style.aspectRatio = String(ratio);
  }
  function chooseQvImage(url){
    var p = PRODUCTS.find(function(x){ return x.id===qvId; });
    if(!p || !url) return;
    var im = document.getElementById("qvImg");
    im.style.opacity = 0;
    if(url === p.img){ applyImgSrc(im, p, "(max-width:720px) 100vw, 460px", 896); }
    else { im.removeAttribute("srcset"); im.removeAttribute("sizes"); im.src = url; }
    im.alt = p.name;
    im.onload = function(){ im.style.opacity = 1; fitQvMedia(im); };
    if(im.complete && im.naturalWidth) fitQvMedia(im);
    Array.prototype.forEach.call(document.querySelectorAll("#qvThumbs .qv-thumb"), function(b){
      b.classList.toggle("on", b.getAttribute("data-qvimg")===url);
    });
  }
  function openQV(id){
    var p = PRODUCTS.find(function(x){return x.id===id;}); if(!p) return;
    qvId = id; qvN = 1;
    var sizes = qvSizesOf(p);
    qvSize = sizes.length===1 ? sizes[0] : "";
    var im = document.getElementById("qvImg"); im.style.opacity = 0; applyImgSrc(im, p, "(max-width:720px) 100vw, 460px", 896); im.alt = p.name; im.style.display = "";
    im.onload = function(){ im.style.opacity = 1; fitQvMedia(im); };
    if(im.complete && im.naturalWidth) fitQvMedia(im);
    renderQvThumbs(p);
    document.getElementById("qvTag").textContent = p.tag||"";
    document.getElementById("qvCat").textContent = catName(p.cat);
    document.getElementById("qvName").textContent = p.name;
    document.getElementById("qvMeta").textContent = p.size || "";
    document.getElementById("qvDesc").textContent = p.desc;
    document.getElementById("qvPrice").textContent = priceLabel(p);
    document.getElementById("qvQty").textContent = "1";
    document.getElementById("qvFabric").textContent = p.fabric||"—";
    document.getElementById("qvFit").textContent = p.fit||"—";
    document.getElementById("qvCare").textContent = p.care||"—";
    renderQvSizes(p);
    updateQvOrder();
    loadReviews(id);
    track("view_product", {id:id});
    qvModal.classList.add("open");
  }
  function closeQV(){ qvModal.classList.remove("open"); }
  /* One scroll lock owner for every overlay; MutationObserver keeps direct
     class changes (route resets, deep links) in sync. */
  (function(){
    var layers=["qvModal","expressModal","sizeModal","coModal","authModal",
      "contactModal","cartDrawer"].map(function(id){ return document.getElementById(id); }).filter(Boolean);
    if(!layers.length || !window.MutationObserver) return;
    function syncLock(){
      var open=false;
      layers.forEach(function(el){ if(el.classList.contains("open")) open=true; });
      document.body.classList.toggle("modal-open", open);
    }
    var mo=new MutationObserver(syncLock);
    layers.forEach(function(el){ mo.observe(el,{attributes:true,attributeFilter:["class"]}); });
    syncLock();
  })();
  document.getElementById("qvClose").addEventListener("click", closeQV);
  qvModal.addEventListener("click", function(e){ if(e.target===qvModal) closeQV(); });
  document.getElementById("qvThumbs").addEventListener("click", function(e){
    var b = e.target.closest("[data-qvimg]");
    if(b) chooseQvImage(b.getAttribute("data-qvimg"));
  });
  document.getElementById("qvInc").addEventListener("click", function(){ qvN=Math.min(qvMax(),qvN+1); updateQvOrder(); });
  document.getElementById("qvDec").addEventListener("click", function(){ qvN=Math.max(1,qvN-1); updateQvOrder(); });
  document.getElementById("qvAdd").addEventListener("click", function(){
    if(!qvId) return;
    var p = PRODUCTS.find(function(x){return x.id===qvId;});
    if(!p) return;
    if(p.stock!==undefined && (parseInt(p.stock,10)||0)<=0){ notify(t("oos"),"error"); return; }
    if(!qvSize){ notify(t("pickSize"),"error"); return; }
    if(p.stock!==undefined && productQty(qvId)+qvN>p.stock){ notify(t("oos"),"error"); return; }
    var f = findLine(qvId, qvSize);
    if(f) f.qty+=qvN; else cart.push({id:qvId, size:qvSize, qty:qvN});
    saveCart(); renderCart(); closeQV(); cartDrawer.classList.add("open");
    notify(t("addedOne", p.name),"success");
    track("add_to_bag",{id:qvId,size:qvSize});
  });
  document.getElementById("qvExpress").addEventListener("click", openExpress);
  function setExErr(id, msg){
    var e = document.getElementById(id); if(!e) return;
    if(msg){ e.textContent=msg; e.hidden=false; } else { e.hidden=true; }
  }
  function submitExpress(){
    if(!exId || exInflight) return;
    var p = productById(exId); if(!p) return;
    var nameEl=document.getElementById("exNameIn"), phoneEl=document.getElementById("exPhoneIn"), comEl=document.getElementById("exCommuneIn"), wsel=document.getElementById("exWilaya");
    var name=nameEl?nameEl.value.trim():"";
    var phone=phoneEl?phoneEl.value.replace(/\D/g,""):"";
    var wcode=wsel?wsel.value:"";
    var comm=comEl?comEl.value.trim():"";
    setExErr("exEName",""); setExErr("exEPhone",""); setExErr("exEWilaya",""); setExErr("exECommune","");
    var ok=true;
    if(name.length<3){ setExErr("exEName", t("errReq")); ok=false; }
    if(!/^0[567]\d{8}$/.test(phone)){ setExErr("exEPhone", t("errPhone")); ok=false; }
    if(!wcode){ setExErr("exEWilaya", t("errReq")); ok=false; }
    if(comm.length<2){ setExErr("exECommune", t("errReq")); ok=false; }
    if(!ok){ notify(t("coFill"),"error"); return; }
    if(p.stock!==undefined && exQty>(parseInt(p.stock,10)||0)){ notify(t("oos"),"error"); return; }
    var m = exMath();
    var payload = {items:[{id:exId,size:exSize,qty:exQty}], name:name, phone:phone, commune:comm,
      wilaya:parseInt(wcode,10), method:m.method, provider:m.prov, promo:"",
      email:authState.email||"", token:apiToken,
      idem:(exIdem || (exIdem = Date.now().toString(36)+Math.random().toString(36).slice(2,10)))};
    var place = document.getElementById("exPlace");
    exInflight=true; if(place) place.disabled=true;
    function done(){ exInflight=false; if(place) place.disabled=false; }
    api("/api/orders","POST",payload).then(function(r){
      done();
      if(r && r.ok){ exIdem = ""; finishExpress(r.num, r.items, r.total, payload); }
      else notify(t("orderFailed"),"error");
    }).catch(function(e){
      done();
      if(e && e.status===409){ exIdem = ""; notify(t("oos"),"error"); syncStock().then(function(){renderProducts();}).catch(function(){}); }
      else if(e && e.status===429){ notify(t("tooFast"),"error"); }
      else if(e && e.status===400){ exIdem = ""; notify(t("coFill"),"error"); }
      else notify(t("orderFailed"),"error");
    });
  }
  function expressDoneWaLink(){
    if(!WHATSAPP_NUMBER || !exDoneData) return "";
    var msg = "Noor — "+t("expressSuccess")+"\n"+t("coOrderNo", exDoneData.num)+"\n"+exDoneData.n+" · "+exDoneData.total;
    return "https://wa.me/"+WHATSAPP_NUMBER+"?text="+encodeURIComponent(msg);
  }
  if(expressModal){
    document.getElementById("exClose").addEventListener("click", closeExpress);
    expressModal.addEventListener("click", function(e){ if(e.target===expressModal) closeExpress(); });
    document.getElementById("exDec").addEventListener("click", function(){ exQty=Math.max(1,exQty-1); updateExpressTotals(); });
    document.getElementById("exInc").addEventListener("click", function(){
      var p=productById(exId); var max=p&&p.stock!==undefined?Math.max(1,parseInt(p.stock,10)||1):99;
      exQty=Math.min(max,exQty+1); updateExpressTotals();
    });
    document.getElementById("exWilaya").addEventListener("change", function(){ setExErr("exEWilaya",""); updateExpressTotals(); });
    document.getElementById("exProvider").addEventListener("change", updateExpressTotals);
    Array.prototype.forEach.call(document.querySelectorAll('input[name="exshipm"]'), function(r){ r.addEventListener("change", updateExpressTotals); });
    document.getElementById("exNameIn").addEventListener("input", function(){ setExErr("exEName",""); });
    document.getElementById("exPhoneIn").addEventListener("input", function(){ setExErr("exEPhone",""); });
    document.getElementById("exCommuneIn").addEventListener("input", function(){ setExErr("exECommune",""); });
    document.getElementById("exPlace").addEventListener("click", submitExpress);
    document.getElementById("exWa").addEventListener("click", function(){
      var link=expressWaLink();
      if(link) window.open(link,"_blank"); else notify(t("expressFailed"),"error");
    });
    document.getElementById("exCont").addEventListener("click", function(){ closeExpress(); closeQV(); window.location.hash="shop"; });
    document.getElementById("exDoneWa").addEventListener("click", function(){
      var link=expressDoneWaLink();
      if(link) window.open(link,"_blank");
    });
  }
  /* ----- Reviews + share ----- */
  function stars(r){ var s=""; for(var i=1;i<=5;i++) s+= i<=r?"★":"☆"; return s; }
  function escapeHtml(s){ return String(s||"").replace(/[&<>"']/g, function(c){ return {"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]; }); }
  function loadReviews(pid){
    var box=document.getElementById("rvList"), avg=document.getElementById("rvAvg"), count=document.getElementById("rvCount");
    if(!box) return;
    box.innerHTML=""; if(avg) avg.textContent="…"; if(count) count.textContent="";
    api("/api/reviews?product="+encodeURIComponent(pid)).then(function(d){
      var list=(d&&d.ok&&d.reviews)?d.reviews:[];
      if(!list.length){ if(avg) avg.textContent=""; if(count) count.textContent=""; box.innerHTML='<div class="rv-item">'+t("noReviews")+"</div>"; return; }
      var sum=list.reduce(function(s,r){return s+(r.rating||0);},0);
      if(avg) avg.textContent=t("avgLine", list.length, (sum/list.length).toFixed(1));
      if(count) count.textContent="("+list.length+")";
      box.innerHTML=list.slice().reverse().map(function(r){
        return '<div class="rv-item"><b>'+escapeHtml(r.name)+' <span class="stars">'+stars(r.rating)+'</span></b><p>'+escapeHtml(r.text)+"</p></div>";
      }).join("");
    }).catch(function(){ if(avg) avg.textContent=""; if(count) count.textContent=""; box.innerHTML='<div class="rv-item">'+t("noReviews")+"</div>"; });
  }
  document.getElementById("rvSend").addEventListener("click", function(){
    var nm=document.getElementById("rvName").value.trim();
    var tx=document.getElementById("rvText").value.trim();
    var rt=parseInt(document.getElementById("rvStars").value,10);
    if(!qvId || nm.length<2 || tx.length<3 || !(rt>=1&&rt<=5)){ notify(t("reviewBad"),"error"); return; }
    api("/api/reviews","POST",{product:qvId,name:nm,rating:rt,text:tx}).then(function(r){
      if(r&&r.ok){ document.getElementById("rvText").value=""; notify(t("reviewThanks"),"success"); loadReviews(qvId); }
      else notify(t("reviewBad"),"error");
    }).catch(function(e){ notify((e&&e.status===429)?t("tooFast"):t("reviewBad"),"error"); });
  });
  document.getElementById("qvShare").addEventListener("click", function(){
    if(!qvId) return;
    var url=location.origin+location.pathname+"#/p/"+qvId;
    function done(){ notify(t("copied"),"success"); }
    function fallback(){ window.prompt("Copy link:", url); }
    if(navigator.clipboard&&navigator.clipboard.writeText){ navigator.clipboard.writeText(url).then(done).catch(function(){ fallback(); }); }
    else fallback();
  });
  /* ----- Checkout ----- */
  var coModal = document.getElementById("coModal");
  var promoApplied = null;
  var coInflight = false;
  var coWilayaSel = document.getElementById("coWilaya");
  function fillWilaya(){
    if(!coWilayaSel || !coModal.classList.contains("open")) return;
    var keep = coWilayaSel.value;
    coWilayaSel.innerHTML = "";
    var o0 = document.createElement("option"); o0.value = ""; o0.textContent = "— "+t("coWilaya")+" —";
    coWilayaSel.appendChild(o0);
    WILAYAS.forEach(function(w, i){
      var o = document.createElement("option"); o.value = (i+1); o.textContent = (i+1)+" — "+w;
      coWilayaSel.appendChild(o);
    });
    if(keep) coWilayaSel.value = keep;
  }
  function shipMethod(){ var r = document.querySelector('input[name="shipm"]:checked'); return r?r.value:"home"; }
  var SHIPDATA = null;
  function shipZoneOf(code){ if(!SHIPDATA||!SHIPDATA.zones) return "rest"; var z=SHIPDATA.zones; for(var k in z){ if(z[k].indexOf(code)>=0) return k; } return "rest"; }
  function coProvider(){ var s=document.getElementById("coProvider"); return (s&&s.value)?s.value:"standard"; }
  function shipFee2(prov, code, method){
    if(SHIPDATA&&SHIPDATA.providers&&SHIPDATA.providers[prov]){
      var tb=SHIPDATA.providers[prov][method==="stop"?"stop":"home"]||{};
      var z=shipZoneOf(code);
      return (tb[z]!==undefined?tb[z]:(tb.rest!==undefined?tb.rest:800));
    }
    return shipFee(code, method);
  }
  function shipFree(prov){
    if(!SHIPDATA||!SHIPDATA.providers||!SHIPDATA.providers[prov]) return true;
    return !!SHIPDATA.providers[prov].free;
  }
  function fillShipping(){
    var s=document.getElementById("coProvider"); if(!s) return;
    var keep=s.value||"standard";
    s.innerHTML="";
    var provs=(SHIPDATA&&SHIPDATA.providers)?SHIPDATA.providers:{standard:{name:"Noor Standard"}};
    Object.keys(provs).forEach(function(k){
      var o=document.createElement("option"); o.value=k; o.textContent=provs[k].name; s.appendChild(o);
    });
    s.value=provs[keep]?keep:"standard";
  }
  function fetchSettings(){
    api("/api/settings").then(function(d){
      if(d&&d.ok){
        WHATSAPP_NUMBER=d.whatsapp||"";
        applyStoreSettings(d);
        updateExpressWhatsApp();
      }
    }).catch(function(){});
    api("/api/shipping").then(function(d){
      if(d&&d.ok&&d.shipping){
        SHIPDATA=d.shipping; fillShipping(); coTotals();
        if(expressModal && expressModal.classList.contains("open")){ fillExpressProviders(); updateExpressTotals(); }
      }
    }).catch(function(){});
  }
  var SOCIAL_FALLBACK = {
    instagram:"https://www.instagram.com/noor.thobehouse",
    facebook:"https://www.facebook.com/noor.thobehouse",
    tiktok:"https://www.tiktok.com/@noor.thobehouse"
  };
  var SITE = { name:"Noor", logo:"", theme:null, banner:null, promos:[{code:"NOOR10",pct:10,active:true}], social:Object.assign({}, SOCIAL_FALLBACK) };
  function applyStoreSettings(d){
    if(d.whatsapp!==undefined) WHATSAPP_NUMBER=d.whatsapp||"";
    if(d.siteName && d.siteName!==SITE.name){
      SITE.name=d.siteName;
      var sn=document.getElementById("siteName"); if(sn) sn.textContent=d.siteName;
    }
    SITE.logo=d.logo||"";
    var im=document.getElementById("siteLogo"), moon=document.getElementById("siteMoon");
    if(im){
      if(SITE.logo){
        if(im.getAttribute("src")!==SITE.logo) im.src=SITE.logo;
        if(im.hidden) im.hidden=false;
        if(moon) moon.style.display="none";
      } else if(!im.hidden || im.getAttribute("src")){
        im.hidden=true; im.removeAttribute("src"); if(moon) moon.style.display="";
      }
    }
    if(d.theme){
      SITE.theme=d.theme;
      /* The stylesheet already ships the default Oud palette. Writing those
         same CSS variables back would invalidate the whole document. */
      var def = d.theme.preset==="oud" &&
        d.theme.shade==="#f5f1e8" && d.theme.fg==="#201814" &&
        d.theme.rose==="#8a6d3b" && d.theme.roseDeep==="#4a3b22" &&
        d.theme.blue==="#2f4f43";
      if(!def) applyTheme(d.theme);
    }
    buildThemeDots();
    if(d.banner){ SITE.banner=d.banner; applyBanner(d.banner); }
    if(d.promos&&d.promos.length){ SITE.promos=d.promos; }
    if(d.social && typeof d.social==="object"){
      SITE.social = {
        instagram:d.social.instagram||"",
        facebook:d.social.facebook||"",
        tiktok:d.social.tiktok||""
      };
    }
    renderContact();
  }
  function contactWaHref(){
    if(!WHATSAPP_NUMBER) return "";
    return "https://wa.me/"+WHATSAPP_NUMBER+"?text="+encodeURIComponent(t("contactWaMsg"));
  }
  function contactWaDisplay(){
    var n=String(WHATSAPP_NUMBER||"").replace(/\D/g,"");
    if(!n) return "";
    if(n.length===12 && n.slice(0,3)==="213") return "+213 "+n.slice(3,6)+" "+n.slice(6,9)+" "+n.slice(9);
    return "+"+n;
  }
  function socialHandle(url){
    try{
      var path = new URL(url).pathname.replace(/\/+$/, "");
      var part = path.split("/").filter(Boolean).pop() || "";
      if(part && part.charAt(0) !== "@") part = "@"+part;
      return part;
    }catch(e){ return ""; }
  }
  function renderContact(){
    if(!contactModal) return;
    var href = contactWaHref();
    var wa = document.getElementById("contactWa");
    if(wa){ wa.href = href || "#"; wa.hidden = !href; }
    var num = document.getElementById("contactWaNumber");
    if(num) num.textContent = contactWaDisplay();
    var fab = document.getElementById("waFab");
    if(fab){ fab.href = href || "#"; fab.hidden = !href; fab.setAttribute("aria-label", t("contactWa")); }
    var box = document.getElementById("contactSocials");
    if(!box) return;
    var socials = SITE.social || {};
    var items = [
      {key:"instagram", name:"Instagram", mark:"IG", url:socials.instagram},
      {key:"facebook", name:"Facebook", mark:"FB", url:socials.facebook},
      {key:"tiktok", name:"TikTok", mark:"TT", url:socials.tiktok}
    ].filter(function(item){ return !!item.url; });
    box.hidden = !items.length;
    box.innerHTML = items.map(function(item){
      var handle = socialHandle(item.url);
      return '<a class="social-btn '+item.key+'" href="'+escapeHtml(item.url)+'" target="_blank" rel="noopener" aria-label="'+escapeHtml(item.name)+'">'+
        '<span class="social-mark" aria-hidden="true">'+item.mark+'</span>'+
        '<span class="social-copy"><b>'+escapeHtml(item.name)+'</b>'+(handle?'<small>'+escapeHtml(handle)+'</small>':'')+'</span>'+
        '<span class="social-arrow" aria-hidden="true">↗</span></a>';
    }).join("");
  }
  function openContact(){
    if(!contactModal) return;
    renderContact();
    contactModal.classList.add("open");
    document.body.classList.add("contact-open");
  }
  function closeContact(){
    if(!contactModal) return;
    contactModal.classList.remove("open");
    document.body.classList.remove("contact-open");
    if(window.location.hash === "#contact"){
      try{ history.replaceState(null, "", window.location.pathname + window.location.search + "#home"); }catch(e){ window.location.hash="home"; }
    }
  }
  function copyContactKit(){
    var s = SITE.social || {};
    var lines = [
      "NOOR THOBE HOUSE",
      "Men's thobes, djellabas and shirts — Algiers, Algeria.",
      "Shop: " + window.location.origin + window.location.pathname,
      "WhatsApp: +" + WHATSAPP_NUMBER,
      s.instagram ? "Instagram: " + s.instagram : "",
      s.facebook ? "Facebook: " + s.facebook : "",
      s.tiktok ? "TikTok: " + s.tiktok : "",
      "Market: all 58 wilayas · DZD · cash on delivery",
      "Languages: English · Français · العربية",
      "Sponsorships: paid collaborations, campaign features and event coverage.",
      "Formats: Reels · Stories · Lookbooks · Event features."
    ].filter(Boolean).join("\n");
    function done(){ notify(t("contactCopied"), "success"); }
    function fallback(){
      var area = document.createElement("textarea");
      area.value = lines; area.setAttribute("readonly", ""); area.style.position="fixed"; area.style.left="-9999px";
      document.body.appendChild(area); area.select();
      try{ document.execCommand("copy"); done(); }catch(e){ notify(t("contactCopied"), "info"); }
      document.body.removeChild(area);
    }
    if(navigator.clipboard && navigator.clipboard.writeText){
      navigator.clipboard.writeText(lines).then(done, fallback);
    } else fallback();
  }
  var APPLIED_THEME_SIG = "";
  function applyTheme(th){
    if(!th) return;
    var sig=[th.preset,th.shade,th.fg,th.rose,th.roseDeep,th.blue].join("|");
    if(sig===APPLIED_THEME_SIG) return;
    APPLIED_THEME_SIG=sig;
    var r=document.documentElement.style;
    if(th.shade) r.setProperty("--shade", th.shade);
    if(th.fg) r.setProperty("--fg", th.fg);
    if(th.rose) r.setProperty("--rose", th.rose);
    if(th.roseDeep) r.setProperty("--rose-deep", th.roseDeep);
    if(th.blue){ r.setProperty("--blue", th.blue); r.setProperty("--blue-hover", th.blue); }
    var mt=document.querySelector('meta[name="theme-color"]');
    if(mt&&th.shade) mt.setAttribute("content", th.shade);
     if(th.preset) document.documentElement.setAttribute("data-preset", th.preset);
  }
  var TH_DOTS=[["oud","linear-gradient(135deg,#f5f1e8 40%,#8a6d3b 41%,#2f4f43 99%)"],["sand","linear-gradient(135deg,#f4ecdd 40%,#d2a24c 41%,#9a6b23 99%)"],["midnight","linear-gradient(135deg,#101418 40%,#8fa3c7 41%,#5b718f 99%)"],["noir","linear-gradient(135deg,#ececec 40%,#b5b5b5 41%,#0a0a0a 99%)"]];
  function activatePreset(name){ var th=THEMES[name]; if(!th) return; applyTheme(th);
    Array.prototype.forEach.call(document.querySelectorAll("#themeDots button"), function(b){ b.classList.toggle("on", b.getAttribute("data-t")===name); }); }
  function buildThemeDots(){ var box=document.getElementById("themeDots"); if(!box||box.children.length) return;
    TH_DOTS.forEach(function(d){ var b=document.createElement("button"); b.type="button"; b.setAttribute("data-t", d[0]);
      b.setAttribute("aria-label", d[0]+" theme"); b.style.background=d[1]; b.title=d[0];
      b.addEventListener("click", function(){ activatePreset(d[0]); try{ storeSet("noor_theme", d[0]); }catch(e){} });
      box.appendChild(b); });
    var saved=null; try{ saved=storeGet("noor_theme", null); }catch(e){}
    if(saved&&THEMES[saved]) activatePreset(saved);
  }
  function applyBanner(bn){
    var b=document.getElementById("eventBanner");
    if(!b) return;
    if(bn&&bn.active&&bn.text){
      var label = bn.text==="Free delivery over 8,000 DZD · Cash on delivery in all 58 wilayas" ? t("bannerDefault") : bn.text;
      if(b.textContent!==label) b.textContent=label;
      var bg=bn.bg||"#171214", fg=bn.fg||"#fff";
      if(b.style.background!==bg) b.style.background=bg;
      if(b.style.color!==fg) b.style.color=fg;
      if(b.hidden) b.hidden=false;
      document.body.classList.add("has-banner");
    } else if(!b.hidden){
      b.hidden=true; document.body.classList.remove("has-banner");
    }
  }
  function findPromo(code){
    code=String(code||"").trim().toUpperCase();
    for(var i=0;i<SITE.promos.length;i++){ if(SITE.promos[i].code===code&&SITE.promos[i].active) return SITE.promos[i]; }
    return null;
  }
  function coMath(){
    var sub = cart.reduce(function(s,i){ var p=PRODUCTS.find(function(x){return x.id===i.id;}); return s+(p?p.price*i.qty:0); },0);
    var disc = promoApplied?Math.round(sub*promoApplied.pct/100):0;
    var code = parseInt(coWilayaSel.value||"0", 10);
    var prov = coProvider();
    var freeOver = (SHIPDATA&&SHIPDATA.freeOver)||8000;
    var ship = ((shipFree(prov)&&sub-disc>=freeOver)||sub===0)?0:shipFee2(prov, code, shipMethod());
    var n = cart.reduce(function(s,i){return s+i.qty;},0);
    return {sub:sub, disc:disc, ship:ship, total:sub-disc+ship, n:n};
  }
  function coTotals(){
    if(!coWilayaSel || !coModal) return;
    var m = coMath();
    document.getElementById("coSubV").textContent = money(m.sub);
    document.getElementById("coDiscRow").hidden = !promoApplied;
    document.getElementById("coDiscV").textContent = "−"+money(m.disc);
    document.getElementById("coShipV").textContent = (m.ship===0 && m.sub>0)?t("free"):money(m.ship);
    document.getElementById("coTotalV").textContent = money(m.total);
    document.getElementById("coItems").innerHTML = cart.map(function(i){
      var p = PRODUCTS.find(function(x){return x.id===i.id;}); if(!p) return "";
      var sz = i.size && i.size!=="One size" ? " · "+t("yourSize")+" "+i.size : "";
      return "<div><span>"+escapeHtml(p.name)+" × "+i.qty+sz+"</span><b>"+money(p.price*i.qty)+"</b></div>";
    }).join("");
  }
  var WHATSAPP_NUMBER = "";
  function waOrderLink(){
    var L = "\n";
    var msg = "Noor — "+t("coTitle")+L+cart.map(function(i){
      var p=PRODUCTS.find(function(x){return x.id===i.id;});
      if(!p) return "";
      var sz = i.size && i.size!=="One size" ? " · "+t("yourSize")+" "+i.size : "";
      return "• "+p.name+" × "+i.qty+sz+" — "+money(p.price*i.qty);
    }).join(L)+L+t("coTotal")+": "+money(coMath().total);
    return "https://wa.me/"+WHATSAPP_NUMBER+"?text="+encodeURIComponent(msg);
  }
  function prefillCustomer(){
    var em=document.getElementById("coEmailIn");
    if(em){ em.value = authState.email || em.value; em.disabled = !!authState.email; }
    var c = customerDetails(); if(!c) return;
    var nameEl=document.getElementById("coNameIn"), phoneEl=document.getElementById("coPhoneIn"), comEl=document.getElementById("coCommuneIn");
    if(nameEl && c.name) nameEl.value = c.name;
    if(phoneEl && c.phone) phoneEl.value = c.phone;
    if(coWilayaSel && c.wilaya) coWilayaSel.value = String(c.wilaya);
    if(comEl && c.commune) comEl.value = c.commune;
    var method = c.method==="stop" ? "stop" : "home";
    Array.prototype.forEach.call(document.querySelectorAll('input[name="shipm"]'), function(r){ r.checked = (r.value===method); });
    var ps=document.getElementById("coProvider");
    if(ps && c.provider){ ps.value=c.provider; if(!ps.value) ps.value="standard"; }
  }
  function openCO(){
    promoApplied = null;
    document.getElementById("coPromoIn").value = "";
    document.getElementById("coForm").hidden = false;
    document.getElementById("coDone").hidden = true;
    coModal.classList.add("open");
    fillWilaya(); fillShipping(); prefillCustomer(); coTotals();
    document.getElementById("coWa").style.display = WHATSAPP_NUMBER?"":"none";
    track("begin_checkout", {});
  }
  function closeCO(){ coModal.classList.remove("open"); }
  document.getElementById("coCancel").addEventListener("click", closeCO);
  coModal.addEventListener("click", function(e){ if(e.target===coModal) closeCO(); });
  document.getElementById("coCont").addEventListener("click", function(){ closeCO(); window.location.hash="home"; });
  document.getElementById("coWa").addEventListener("click", function(){
    if(!WHATSAPP_NUMBER || !cart.length) return;
    window.open(waOrderLink(), "_blank");
  });
  coWilayaSel.addEventListener("change", coTotals);
  document.getElementById("coProvider").addEventListener("change", coTotals);
  Array.prototype.forEach.call(document.querySelectorAll('input[name="shipm"]'), function(r){ r.addEventListener("change", coTotals); });
  document.getElementById("coPromoBtn").addEventListener("click", function(){
    var v = document.getElementById("coPromoIn").value.trim().toUpperCase();
    var p = findPromo(v);
    if(p){ promoApplied = p; notify(t("coPromoOk").replace("−10%", "−"+p.pct+"%"),"success"); }
    else { promoApplied = null; notify(t("coPromoBad"),"error"); }
    coTotals();
  });
  var coIdem = "", exIdem = "";
  document.getElementById("coPlace").addEventListener("click", function(){
    var nameEl = document.getElementById("coNameIn"), phoneEl = document.getElementById("coPhoneIn"), comEl = document.getElementById("coCommuneIn");
    var name = nameEl.value.trim();
    var phone = phoneEl.value.replace(/\D/g,"");
    var wcode = coWilayaSel.value;
    var comm = comEl.value.trim();
    function setErr(id, msg){ var e=document.getElementById(id); if(!e) return; if(msg){ e.textContent=msg; e.hidden=false; } else { e.hidden=true; } }
    setErr("eName",""); setErr("ePhone",""); setErr("eWilaya",""); setErr("eCommune","");
    var ok = true;
    if(name.length<3){ setErr("eName", t("errReq")); ok=false; }
    if(!/^0[567]\d{8}$/.test(phone)){ setErr("ePhone", t("errPhone")); ok=false; }
    if(!wcode){ setErr("eWilaya", t("errReq")); ok=false; }
    if(comm.length<2){ setErr("eCommune", t("errReq")); ok=false; }
    if(!ok){ notify(t("coFill"),"error"); return; }
    if(coInflight) return;
    coInflight = true;
    var placeBtn = document.getElementById("coPlace"); placeBtn.disabled = true;
    var coEmailEl = document.getElementById("coEmailIn");
    // One idempotency key per checkout attempt: retrying after a dropped
    // connection can never create the order twice.
    if(!coIdem) coIdem = Date.now().toString(36)+Math.random().toString(36).slice(2,10);
    var payload = {items:cart.map(function(i){ return {id:i.id, size:i.size||"", qty:i.qty}; }), name:name, phone:phone, commune:comm, wilaya:parseInt(wcode,10), method:shipMethod(), provider:coProvider(), promo:(promoApplied?promoApplied.code:""), email:authState.email||(coEmailEl?coEmailEl.value.trim():""), token:apiToken, idem:coIdem};
    saveCustomerDetails(payload);
    var m = coMath();
    function coDone(){ coInflight = false; placeBtn.disabled = false; }
    api("/api/orders","POST",payload).then(function(r){
      coDone();
      if(r && r.ok){ apiMode = true; coIdem = ""; afterOrder(r.num, r.items, r.total); }
      else notify(t("orderFailed"),"error");
    }).catch(function(e){
      coDone();
      // Never show "Order confirmed" unless the shop really received it.
      if(e && e.status===409){ coIdem = ""; notify(t("oos"),"error"); syncStock().then(function(){ renderProducts(); renderCart(); }).catch(function(){}); }
      else if(e && e.status===429){ notify(t("tooFast"),"error"); }
      else if(e && e.status===400){ coIdem = ""; notify(t("coFill"),"error"); }
      else notify(t("orderFailed"),"error");
    });
  });
  /* ----- Language switcher + misc ----- */
  function updateMiniAdds(){
    Array.prototype.forEach.call(document.querySelectorAll(".mini-add"), function(b){ b.textContent = t("addBag"); });
  }
  Array.prototype.forEach.call(document.querySelectorAll(".langseg button"), function(b){
    b.addEventListener("click", function(){ applyLang(b.getAttribute("data-lang")); });
  });
  document.addEventListener("keydown", function(e){
    if(e.key!=="Escape") return;
    closeExpress(); closeQV(); closeCO(); closeSizeGuide(); closeContact(); cartDrawer.classList.remove("open");
    var pm=document.getElementById("prodModal"); if(pm) pm.classList.remove("open");
  });
  /* ----- Focus trap + return-focus for modals (WCAG 2.4.3) ----- */
  (function(){
    var lastFocus = null;
    document.addEventListener("keydown", function(e){
      if(e.key!=="Tab") return;
      var opens = document.querySelectorAll(".modal-backdrop.open");
      var open = opens.length ? opens[opens.length-1] : null;
      if(!open) return;
      var els = open.querySelectorAll("button, input, select, textarea, a[href]");
      els = Array.prototype.filter.call(els, function(el){ return !el.disabled && el.getClientRects().length; });
      if(!els.length) return;
      var first = els[0], last = els[els.length-1];
      if(e.shiftKey && document.activeElement===first){ last.focus(); e.preventDefault(); }
      else if(!e.shiftKey && document.activeElement===last){ first.focus(); e.preventDefault(); }
    });
    ["authModal","qvModal","expressModal","coModal","prodModal","sizeModal","contactModal"].forEach(function(id){
      var m = document.getElementById(id);
      if(!m || !window.MutationObserver) return;
      new MutationObserver(function(){
        if(m.classList.contains("open")){
          lastFocus = document.activeElement;
          var f = m.querySelector("input, select, textarea, button");
          if(f) setTimeout(function(){ try{ f.focus(); }catch(e){} }, 80);
        } else if(lastFocus && lastFocus.focus){
          try{ lastFocus.focus(); }catch(e){} lastFocus = null;
        }
      }).observe(m, {attributes:true, attributeFilter:["class"]});
    });
  })();
  /* ----- Magnetic buttons (fine pointers, motion-safe) ----- */
  (function(){
    if(!window.matchMedia) return;
    if(matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    if(matchMedia("(hover: none)").matches) return;
    Array.prototype.forEach.call(document.querySelectorAll(".pill"), function(btn){
      var tx=0, ty=0, cx=0, cy=0, raf=null, rect=null, rectAt=0;
      function tick(){
        cx+=(tx-cx)*0.18; cy+=(ty-cy)*0.18;
        btn.style.transform="translate3d("+cx.toFixed(2)+"px,"+cy.toFixed(2)+"px,0)";
        if(Math.abs(tx-cx)>0.1||Math.abs(ty-cy)>0.1){ raf=requestAnimationFrame(tick); }
        else { raf=null; if(!tx&&!ty) btn.style.transform=""; }
      }
      btn.addEventListener("pointermove", function(e){
        var now=(window.performance && performance.now) ? performance.now() : Date.now();
        if(!rect || now-rectAt>400){ rect=btn.getBoundingClientRect(); rectAt=now; }
        tx=(e.clientX-(rect.left+rect.width/2))*0.28; ty=(e.clientY-(rect.top+rect.height/2))*0.28;
        if(!raf) raf=requestAnimationFrame(tick);
      }, {passive:true});
      btn.addEventListener("pointerleave", function(){ rect=null; tx=0; ty=0; if(!raf) raf=requestAnimationFrame(tick); }, {passive:true});
    });
  })();
  /* ----- Image shimmer watcher ----- */
  function watchImgs(){
    Array.prototype.forEach.call(document.querySelectorAll(".product-img:not([data-w])"), function(img){
      img.setAttribute("data-w","1");
      function done(){ var c=img.closest(".card"); if(c) c.classList.add("img-ok"); }
      if(img.complete && img.naturalWidth) done();
      else { img.addEventListener("load", done, {once:true}); img.addEventListener("error", done, {once:true}); }
    });
  }
  /* ----- Backend API (graceful local fallback) ----- */
  var apiMode = false, apiToken = storeGet("noor_token", "");
  function api(path, method, body, headers){
    var h = Object.assign({"Content-Type":"application/json"}, headers||{});
    return fetch(path, {method:method||"GET", headers:h, body:body?JSON.stringify(body):undefined}).then(function(r){ if(!r.ok){ var e=new Error("api"); e.status=r.status; throw e; } return r.json(); });
  }
  function track(ev, d){
    try{ fetch("/api/track",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({ev:ev,d:d||{}})}).catch(function(){}); }catch(e){}
  }
  function syncStock(){
    return api("/api/products").then(function(d){
      if(d && d.ok && Array.isArray(d.products)){
        apiMode = true;
        var list = d.products.filter(function(s){ return s && s.id && s.name && s.price; });
        PRODUCTS.length = 0;
        list.forEach(function(s){ PRODUCTS.push(s); });
        sanitizeCart();
      }
    }).catch(function(){ apiMode = false; });
  }
  function apiAuth(email, pass){
    function authed(r){
      if(r && r.ok){ apiMode = true; apiToken = r.token; storeSet("noor_token", r.token); return true; }
      return false;
    }
    return api("/api/auth/login","POST",{email:email,pass:pass}).then(function(r){
      if(authed(r)) return true;
      notify(t("authErr"),"error");
      return false;
    }).catch(function(e){
      if(e && e.status===404 && confirm(t("regConfirm", email))){
        return api("/api/auth/register","POST",{email:email,pass:pass}).then(authed).catch(function(){
          notify(t("authErr"),"error");
          return false;
        });
      }
      if(!e || e.status!==404) notify(t("authErr"),"error");
      return false;
    });
  }
  function orderInfoHtml(num, n, total){
    return '<span class="co-done-line">'+t("coOrderNo", num)+'</span>'+
      '<span class="co-done-line"><bdi dir="ltr">'+n+' · '+total+'</bdi></span>';
  }
  function afterOrder(num, n, total){
    cart = []; promoApplied = null; saveCart(); renderCart(); refreshOrders();
    syncStock().then(function(){ renderProducts(); }).catch(function(){});
    document.getElementById("coForm").hidden = true;
    document.getElementById("coDone").hidden = false;
    document.getElementById("coDoneInfo").innerHTML = orderInfoHtml(num, n, total);
    notify(t("coPlaced", n, total),"success"); track("order_placed",{n:n});
  }
  /* ----- Admin ----- */
  var admToken = "";
  try{ admToken = sessionStorage.getItem("noor_admin") || ""; }catch(e){ admToken = ""; }
  var ADM_PAGES = { overview:"Overview", orders:"Orders", products:"Products", storefront:"Storefront", offers:"Discounts", settings:"Settings" };
  var ADM_STATUS = { pending:"New", confirmed:"Confirmed", shipped:"Shipped", delivered:"Delivered", cancelled:"Cancelled" };
  var ADM_NEXT = {
    pending:[["Confirm order","confirmed"],["Cancel","cancelled"]],
    confirmed:[["Mark as shipped","shipped"],["Cancel","cancelled"]],
    shipped:[["Mark as delivered","delivered"],["Cancel","cancelled"]],
    delivered:[], cancelled:[]
  };
  var admTab = "overview";
  var ADM = { orders:[], products:[], stats:null, settings:null, shipping:null, images:null };
  var admOrderFilter = "all", admOrderQ = "", admProductQ = "", admLoaded = false;
  var admSelProducts = {}, admVisibleProducts = [];
  var pdImages = [], pdUploading = 0;

  function admSaveToken(tok){
    admToken = tok || "";
    try{ if(admToken) sessionStorage.setItem("noor_admin", admToken); else sessionStorage.removeItem("noor_admin"); }catch(e){}
  }
  function adminShowLogin(){
    var l=document.getElementById("adminLogin"), p=document.getElementById("adminPanel");
    if(l) l.hidden=false;
    if(p) p.hidden=true;
  }
  function adminShowPanel(){
    var l=document.getElementById("adminLogin"), p=document.getElementById("adminPanel");
    if(l) l.hidden=true;
    if(p) p.hidden=false;
  }
  function admApi(path, method, body){
    return api(path, method||"GET", body, {"Authorization":"Bearer "+admToken}).catch(function(e){
      if(e && e.status===401){ admSaveToken(""); adminShowLogin(); }
      throw e;
    });
  }
  function admSetLoading(on){
    var el=document.getElementById("admLoading"); if(el) el.hidden=!on;
  }
  function admSetDate(){
    var el=document.getElementById("admDate"); if(!el) return;
    el.textContent = new Date().toLocaleDateString(undefined,{weekday:"long", day:"numeric", month:"long"});
  }
  function admRelative(ts){
    ts = parseInt(ts,10)||0; if(!ts) return "";
    var s = Math.max(0, Math.floor(Date.now()/1000 - ts));
    if(s<60) return "just now";
    if(s<3600) return Math.floor(s/60)+" min ago";
    if(s<86400) return Math.floor(s/3600)+" h ago";
    if(s<604800) return Math.floor(s/86400)+" d ago";
    return new Date(ts*1000).toLocaleDateString(undefined,{day:"numeric",month:"short"});
  }
  function wilayaName(w){
    var n = parseInt(w,10);
    return (n>=1 && n<=58) ? WILAYAS[n-1] : (w ? ("Wilaya "+w) : "Wilaya");
  }
  function providName(key){
    var p = ADM.shipping && ADM.shipping.providers && ADM.shipping.providers[key];
    return p ? p.name : (key||"Noor Standard");
  }
  function admKpi(label, value, sub, tone){
    return '<div class="adm-kpi'+(tone?" adm-kpi-"+tone:"")+'"><span>'+escapeHtml(label)+'</span><b>'+escapeHtml(String(value))+'</b><small>'+escapeHtml(sub)+'</small></div>';
  }
  function orderStatus(o){ return o.status || "pending"; }
  function orderPhone(o){
    var ph = String(o.phone||"").replace(/\D/g,"");
    if(ph.charAt(0)==="0") ph = "213"+ph.slice(1);
    return ph;
  }
  function orderWa(o){
    var msg = "Noor: hello "+(o.name||"")+", your order "+(o.num||"")+" is "+ADM_STATUS[orderStatus(o)].toLowerCase()+".";
    return "https://wa.me/"+orderPhone(o)+"?text="+encodeURIComponent(msg);
  }

  function adminLoad(){
    if(!admToken){ adminShowLogin(); return; }
    admSetLoading(true);
    return Promise.all([
      admApi("/api/admin/stats").catch(function(){ return null; }),
      admApi("/api/admin/orders").catch(function(){ return null; }),
      api("/api/products").catch(function(){ return null; }),
      api("/api/settings").catch(function(){ return null; }),
      api("/api/shipping").catch(function(){ return null; })
    ]).then(function(r){
      admSetLoading(false);
      if(!admToken) return;
      if(r[0] && r[0].ok) ADM.stats = r[0];
      if(r[1] && r[1].ok && r[1].orders) ADM.orders = r[1].orders;
      if(r[2] && r[2].ok && Array.isArray(r[2].products)) ADM.products = r[2].products;
      if(r[3] && r[3].ok) ADM.settings = r[3];
      if(r[4] && r[4].ok) ADM.shipping = r[4].shipping;
      PRODUCTS.length = 0;
      (ADM.products||[]).forEach(function(p){ PRODUCTS.push(p); });
      admLoaded = true;
      fillAdminForms();
      renderAdmTab();
      updateOrdersBadge();
      admSecLoad();
    }).catch(function(){ admSetLoading(false); });
  }
  function updateOrdersBadge(){
    var badge = document.getElementById("admOrdersBadge");
    if(!badge) return;
    var n = ADM.orders.filter(function(o){ return orderStatus(o)==="pending"; }).length;
    badge.textContent = n;
    badge.hidden = !n;
  }
  function renderAdmin(){
    if(!admToken){ adminShowLogin(); return; }
    adminShowPanel();
    admSetDate();
    renderPresets();
    adminLoad();
  }
  function setAdmTab(tab){
    admTab = tab;
    Array.prototype.forEach.call(document.querySelectorAll("[data-adm-tab]"), function(b){
      b.classList.toggle("on", b.getAttribute("data-adm-tab")===tab);
    });
    Array.prototype.forEach.call(document.querySelectorAll("[data-adm-panel]"), function(p){
      p.classList.toggle("on", p.getAttribute("data-adm-panel")===tab);
    });
    var title = document.getElementById("admPageTitle");
    if(title) title.textContent = ADM_PAGES[tab] || "Overview";
    renderAdmTab();
    var main = document.querySelector(".adm-main");
    if(main) main.scrollTop = 0;
  }
  function renderAdmTab(){
    if(admTab==="overview") renderOverview();
    else if(admTab==="orders") renderAdmOrders();
    else if(admTab==="products") renderAdmProducts();
    else if(admTab==="offers") renderPromos();
    else if(admTab==="settings") renderAdmSettings();
  }

  /* Overview */
  function renderOverview(){
    var st = ADM.stats || {};
    var orders = ADM.orders || [];
    var pending = orders.filter(function(o){ return orderStatus(o)==="pending"; }).length;
    var confirmed = orders.filter(function(o){ return orderStatus(o)==="confirmed"; }).length;
    var low = st.lowStock || [];

    var kpis = document.getElementById("admStats");
    if(kpis){
      kpis.innerHTML =
        admKpi("Revenue", st.revenue||"0 DZD", (st.paidOrders||0)+" orders · "+(st.itemsSold||0)+" items sold", "money")+
        admKpi("To confirm", pending, confirmed ? confirmed+" confirmed, waiting to ship" : "Call the customer, then confirm", "urgent")+
        admKpi("Average basket", st.avgBasket||"0 DZD", "per order", "basket")+
        admKpi("Low stock", low.length, (st.outOfStock||0)+" out of stock", low.length?"warn":"ok");
    }

    var alerts = document.getElementById("admAlerts");
    if(alerts){
      var items = [];
      if(pending) items.push({tone:"warn", text:(pending===1?"1 new order needs a confirmation call":pending+" new orders need a confirmation call"), label:"Review orders", goto:"orders"});
      if(low.length) items.push({tone:"warn", text:(low.length===1?"1 piece is low on stock":low.length+" pieces are low on stock"), label:"Restock", goto:"products"});
      if(!items.length) items.push({tone:"ok", text:"You are all caught up. New orders will appear here.", label:"", goto:""});
      alerts.innerHTML = items.map(function(a){
        return '<div class="adm-alert adm-alert-'+a.tone+'"><span>'+escapeHtml(a.text)+'</span>'+(a.goto?'<button type="button" class="adm-link-btn" data-goto="'+a.goto+'">'+escapeHtml(a.label)+' →</button>':"")+'</div>';
      }).join("");
    }

    var chart = document.getElementById("admChart");
    if(chart){
      var days = st.byDay || [];
      var total = days.reduce(function(s,d){ return s+(parseInt(d.revenue,10)||0); },0);
      if(!days.length || !total){
        chart.innerHTML = '<div class="adm-empty">No sales in the last 14 days yet. Your first order will show up here.</div>';
      } else {
        var max = 1;
        days.forEach(function(d){ if((parseInt(d.revenue,10)||0)>max) max=parseInt(d.revenue,10); });
        chart.innerHTML = '<div class="adm-bars">'+days.map(function(d,i){
          var rev = parseInt(d.revenue,10)||0;
          var h = Math.max(3, Math.round(rev/max*100));
          var today = i===days.length-1;
          return '<div class="adm-bar'+(today?" today":"")+'" title="'+escapeHtml(d.day)+' · '+d.orders+' orders · '+escapeHtml(money(rev))+'"><i style="height:'+h+'%"></i><span>'+escapeHtml(String(d.day).slice(-2))+'</span></div>';
        }).join("")+'</div>';
      }
    }

    var wil = document.getElementById("admWilayas");
    if(wil){
      var tops = (st.topWilayas||[]);
      wil.innerHTML = tops.length ? tops.map(function(w,i){
        var pct = Math.round(w.orders/(tops[0].orders||1)*100);
        return '<div class="adm-rank-row"><span class="adm-rank-n">'+(i+1)+'</span><div class="adm-rank-main"><b>'+escapeHtml(wilayaName(w.wilaya))+'</b><i><span style="width:'+pct+'%"></span></i></div><span class="adm-rank-v">'+w.orders+'</span></div>';
      }).join("") : '<div class="adm-empty">No orders yet.</div>';
    }

    var recent = document.getElementById("admRecentOrders");
    if(recent){
      recent.innerHTML = orders.length ? orders.slice(0,5).map(function(o){
        var stt = orderStatus(o);
        return '<button type="button" class="adm-mini-order" data-order="'+escapeHtml(o.num)+'"><span class="adm-mini-main"><b>'+escapeHtml(o.name||o.num)+'</b><small>'+escapeHtml(o.num)+' · '+admRelative(o.ts)+'</small></span><span class="adm-mini-right"><b>'+escapeHtml(o.total||"")+'</b><span class="adm-chip adm-chip-'+stt+'">'+ADM_STATUS[stt]+'</span></span></button>';
      }).join("") : '<div class="adm-empty">No orders yet. Share your store link to get the first one.</div>';
    }

    var lowBox = document.getElementById("admLowStock");
    if(lowBox){
      lowBox.innerHTML = low.length ? low.map(function(p){
        var s = parseInt(p.stock,10)||0;
        return '<div class="adm-stock-row"><span>'+escapeHtml(p.name)+'</span><b class="'+(s<=0?"is-out":"")+'">'+(s<=0?"Out of stock":s+" left")+'</b></div>';
      }).join("") : '<div class="adm-empty">Stock looks healthy.</div>';
    }
  }

  /* Orders */
  function renderAdmOrders(){
    var box = document.getElementById("admOrderList");
    if(!box) return;
    var q = admOrderQ.toLowerCase();
    var list = ADM.orders.filter(function(o){
      var st = orderStatus(o);
      if(admOrderFilter!=="all" && st!==admOrderFilter) return false;
      if(q){
        var hay = [o.num,o.name,o.phone,o.commune,o.email].join(" ").toLowerCase();
        (o.lines||[]).forEach(function(l){ hay += " "+(l.name||"")+" "+(l.size||""); });
        if(hay.indexOf(q)<0) return false;
      }
      return true;
    });
    if(!list.length){
      box.innerHTML = '<div class="adm-empty-card"><b>No orders here.</b><span>'+(ADM.orders.length?"Try another filter or clear the search.":"New orders will appear here as soon as a customer checks out.")+'</span></div>';
      return;
    }
    box.innerHTML = list.map(admOrderCard).join("");
  }
  function admOrderCard(o){
    var st = orderStatus(o);
    var lines = (o.lines && o.lines.length) ? o.lines.map(function(l){
      return '<div class="adm-line"><span class="adm-line-qty">'+Number(l.qty||1)+'×</span><span class="adm-line-name">'+escapeHtml(l.name||l.id)+'</span><b>'+escapeHtml(l.size||"One size")+'</b></div>';
    }).join("") : '<div class="adm-line adm-line-missing">Item details were not stored for this order.</div>';
    var actions = ADM_NEXT[st] || [];
    var method = (o.method==="stop") ? "Stopdesk pickup" : "Home delivery";
    return '<article class="adm-order-card" data-num="'+escapeHtml(o.num)+'">'+
      '<header class="adm-order-head"><div class="adm-order-id"><b>'+escapeHtml(o.num)+'</b><span class="adm-chip adm-chip-'+st+'">'+ADM_STATUS[st]+'</span></div><div class="adm-order-sum"><b>'+escapeHtml(o.total||"")+'</b><span>'+admRelative(o.ts)+'</span></div></header>'+
      '<div class="adm-order-body">'+
        '<div class="adm-order-cust"><b>'+escapeHtml(o.name||"—")+'</b>'+
          '<a href="tel:'+escapeHtml(o.phone||"")+'">'+escapeHtml(o.phone||"")+'</a>'+
          '<span>'+escapeHtml(wilayaName(o.wilaya))+(o.commune?' · '+escapeHtml(o.commune):'')+'</span>'+
          '<span>'+escapeHtml(method)+' · '+escapeHtml(providName(o.provider))+'</span>'+
        '</div>'+
        '<div class="adm-order-lines">'+lines+'</div>'+
      '</div>'+
      '<footer class="adm-order-foot"><div class="adm-order-links"><a class="adm-btn adm-btn-ghost" target="_blank" rel="noopener" href="'+orderWa(o)+'">WhatsApp</a>'+(o.phone?'<a class="adm-btn adm-btn-ghost" href="tel:'+escapeHtml(o.phone)+'">Call</a>':'')+'</div>'+
        '<div class="adm-order-actions">'+actions.map(function(a){
          return '<button type="button" class="adm-btn '+(a[1]==="cancelled"?"adm-btn-danger-ghost":"adm-btn-primary")+'" data-ostatus="'+a[1]+'" data-onum="'+escapeHtml(o.num)+'">'+escapeHtml(a[0])+'</button>';
        }).join("")+(st==="delivered"?'<span class="adm-done">✓ Delivered</span>':"")+(st==="cancelled"?'<span class="adm-done">Cancelled</span>':"")+'</div>'+
      '</footer></article>';
  }
  function admUpdateStatus(num, status){
    if(status==="cancelled" && !confirm("Cancel order "+num+"? This cannot be undone.")) return;
    admApi("/api/admin/status","POST",{token:admToken,num:num,status:status}).then(function(r){
      if(r && r.ok){ notify("Order "+num+" updated","success"); adminLoad(); }
      else notify("Could not update this order","error");
    }).catch(function(e){
      if(e && e.status===401) return;
      if(e && e.status===409) notify("That step is not allowed for this order.","error");
      else notify("Could not update this order","error");
    });
  }

  /* Products */
  function renderAdmProducts(){
    var box = document.getElementById("admProductList");
    if(!box) return;
    var q = admProductQ.toLowerCase();
    var list = (ADM.products||[]).filter(function(p){
      if(!q) return true;
      return (p.name+" "+p.cat+" "+(p.sizes||[]).join(" ")+" "+(p.fabric||"")).toLowerCase().indexOf(q)>=0;
    });
    var visible = {};
    list.forEach(function(p){ visible[p.id] = 1; });
    Object.keys(admSelProducts).forEach(function(id){ if(!visible[id]) delete admSelProducts[id]; });
    admVisibleProducts = list.map(function(p){ return p.id; });
    if(!list.length){
      box.innerHTML = '<div class="adm-empty-card"><b>No products found.</b><span>Try another search, or add a new piece.</span></div>';
      renderAdmBulkBar();
      return;
    }
    box.innerHTML = list.map(function(p){
      var stock = parseInt(p.stock,10)||0;
      var tone = stock<=0 ? "adm-chip-cancelled" : (stock<=5 ? "adm-chip-pending" : "adm-chip-confirmed");
      var checked = !!admSelProducts[p.id];
      return '<article class="adm-prod'+(checked?' selected':'')+'">'+
        '<label class="adm-prod-check"><input type="checkbox" data-pcheck="'+escapeHtml(p.id)+'"'+(checked?' checked':'')+' aria-label="Select '+escapeHtml(p.name)+'"></label>'+
        '<img src="'+escapeHtml(p.img)+'" alt="" loading="lazy">'+
        '<div class="adm-prod-main"><div class="adm-prod-title"><b>'+escapeHtml(p.name)+'</b><span class="adm-chip '+tone+'">'+(stock<=0?"Out of stock":stock+" in stock")+'</span></div>'+
        '<span>'+escapeHtml(catName(p.cat))+' · '+escapeHtml(money(p.price))+'</span>'+
        '<small>'+escapeHtml((p.sizes||["One size"]).join(" · "))+(p.fabric?" · "+escapeHtml(p.fabric):"")+'</small></div>'+
        '<div class="adm-prod-actions"><button type="button" class="adm-btn adm-btn-ghost" data-pedit="'+escapeHtml(p.id)+'">Edit</button><button type="button" class="adm-btn adm-btn-danger-ghost" data-pdel="'+escapeHtml(p.id)+'">Delete</button></div>'+
      '</article>';
    }).join("");
    renderAdmBulkBar();
  }
  function renderAdmBulkBar(){
    var bar = document.getElementById("admProdBulkBar");
    if(!bar) return;
    var selected = Object.keys(admSelProducts).filter(function(id){ return admSelProducts[id]; });
    var count = selected.length;
    bar.hidden = !admVisibleProducts.length;
    var countEl = document.getElementById("admProdSelCount");
    if(countEl) countEl.textContent = count+" selected";
    var del = document.getElementById("admProdBulkDelete");
    if(del) del.disabled = count===0;
    var all = document.getElementById("admProdSelectAll");
    if(all){
      all.disabled = !admVisibleProducts.length;
      all.checked = admVisibleProducts.length>0 && count===admVisibleProducts.length;
      all.indeterminate = count>0 && count<admVisibleProducts.length;
    }
  }
  function clearAdmProductSelection(){ admSelProducts = {}; renderAdmProducts(); }
  function sizeLabelOf(sizes){
    if(!sizes.length) return "One size";
    if(sizes.length===1) return sizes[0];
    if(sizes.join(",")==="52,54,56,58") return "Sizes 52–58";
    return sizes.join(" · ");
  }
  function pdSetStatus(msg, kind){
    var el = document.getElementById("pdImgHint");
    if(!el) return;
    el.textContent = msg;
    el.className = "pd-img-hint" + (kind ? " " + kind : "");
  }
  function renderPdImages(){
    var grid = document.getElementById("pdImgGrid");
    if(!grid) return;
    grid.innerHTML = pdImages.map(function(url, i){
      return '<div class="pd-thumb'+(i===0?' cover':'')+'" data-pdimg="'+escapeHtml(url)+'" role="button" tabindex="0" aria-label="Make this photo the cover">'+
        '<img src="'+escapeHtml(url)+'" alt="">'+
        (i===0?'<span class="pd-thumb-cover">Cover</span>':'')+
        '<button type="button" class="pd-thumb-x" data-pdremove="'+escapeHtml(url)+'" aria-label="Remove photo">×</button>'+
      '</div>';
    }).join("");
    var hidden = document.getElementById("pdImg");
    if(hidden) hidden.value = pdImages[0] || "";
    var pick = document.getElementById("pdImgPick");
    if(pick) pick.disabled = pdUploading > 0;
    var sel = document.getElementById("pdImgExisting");
    if(sel) sel.disabled = pdUploading > 0 || sel.options.length <= 1;
    var save = document.getElementById("pdSave");
    if(save) save.disabled = pdUploading > 0;
  }
  function loadPdImageLibrary(){
    var sel = document.getElementById("pdImgExisting");
    if(!sel) return;
    function fill(){
      sel.innerHTML = '<option value="">Add an existing image…</option>';
      (ADM.images || []).forEach(function(f){
        var name = String(f || "");
        var url = name.indexOf("images/") === 0 ? name : "images/" + name;
        if(!name || pdImages.indexOf(url) >= 0) return;
        var o = document.createElement("option");
        o.value = url; o.textContent = name.replace("images/", "");
        sel.appendChild(o);
      });
      sel.disabled = pdUploading > 0 || sel.options.length <= 1;
    }
    if(ADM.images && ADM.images.length){ fill(); return; }
    admApi("/api/admin/images","POST",{token:admToken}).then(function(d){
      ADM.images = (d && d.ok && d.images && d.images.length) ? d.images : (ADM.products||[]).map(function(p){ return String(p.img||"").replace("images/",""); });
      fill();
    }).catch(function(){
      ADM.images = (ADM.products||[]).map(function(p){ return String(p.img||"").replace("images/",""); });
      fill();
    });
  }
  function pdBlobToData(blob){
    return new Promise(function(resolve, reject){
      var reader = new FileReader();
      reader.onload = function(){ resolve(String(reader.result || "")); };
      reader.onerror = function(){ reject(new Error("read")); };
      reader.readAsDataURL(blob);
    });
  }
  function pdCompressImage(file){
    return new Promise(function(resolve, reject){
      if(!file || (file.type && file.type.indexOf("image/") !== 0)){ reject(new Error("type")); return; }
      var reader = new FileReader();
      reader.onerror = function(){ reject(new Error("read")); };
      reader.onload = function(){
        var img = new Image();
        function fail(){ reject(new Error("decode")); }
        img.onerror = fail;
        img.onload = function(){
          try{
            var sw = img.naturalWidth || img.width, sh = img.naturalHeight || img.height;
            if(!sw || !sh){ fail(); return; }
            var scale = Math.min(1, 1400 / Math.max(sw, sh));
            var cw = Math.max(1, Math.round(sw * scale)), ch = Math.max(1, Math.round(sh * scale));
            var canvas = document.createElement("canvas");
            canvas.width = cw; canvas.height = ch;
            var ctx = canvas.getContext("2d");
            if(!ctx){ fail(); return; }
            ctx.fillStyle = "#fff"; ctx.fillRect(0, 0, cw, ch);
            ctx.drawImage(img, 0, 0, cw, ch);
            function finish(blob){
              if(!blob){ reject(new Error("encode")); return; }
              if(blob.size > 1500000){
                canvas.toBlob(function(smaller){
                  if(!smaller || smaller.size > 1500000){ reject(new Error("size")); return; }
                  pdBlobToData(smaller).then(function(data){ resolve({filename:file.name || "product.jpg", data:data}); }).catch(reject);
                }, "image/jpeg", 0.68);
                return;
              }
              pdBlobToData(blob).then(function(data){ resolve({filename:file.name || "product.jpg", data:data}); }).catch(reject);
            }
            if(canvas.toBlob){ canvas.toBlob(finish, "image/jpeg", 0.82); }
            else { resolve({filename:file.name || "product.jpg", data:canvas.toDataURL("image/jpeg", 0.82)}); }
          }catch(e){ fail(); }
        };
        img.src = String(reader.result || "");
      };
      reader.readAsDataURL(file);
    });
  }
  function uploadPdFiles(files){
    var queue = Array.prototype.slice.call(files || []).filter(function(f){
      return f && (!f.type || f.type.indexOf("image/") === 0);
    });
    var room = 8 - pdImages.length;
    if(room <= 0){ pdSetStatus("You can add up to 8 photos per product.", "error"); return; }
    queue = queue.slice(0, room);
    if(!queue.length){ pdSetStatus("Choose JPG, PNG or WebP photos.", "error"); return; }
    var total = queue.length, done = 0, failed = 0;
    pdUploading = total;
    renderPdImages();
    function next(){
      if(!queue.length){
        pdUploading = 0;
        renderPdImages();
        loadPdImageLibrary();
        if(failed) pdSetStatus("Added " + done + " photo" + (done===1?"":"s") + "; " + failed + " could not be uploaded. Use JPG, PNG or WebP.", "error");
        else pdSetStatus("Photos added. The first is the cover — tap another photo to change it.", "");
        return;
      }
      var file = queue.shift();
      pdSetStatus("Uploading " + (done + failed + 1) + " of " + total + "…", "busy");
      pdCompressImage(file).then(function(payload){
        return admApi("/api/admin/product-image","POST",{token:admToken, filename:payload.filename, data:payload.data});
      }).then(function(r){
        if(!r || !r.ok || !r.img) throw new Error("upload");
        if(pdImages.indexOf(r.img) < 0) pdImages.push(r.img);
        var uploadedName = String(r.img).replace(/^images\//, "");
        if(ADM.images && ADM.images.indexOf(uploadedName) < 0) ADM.images.push(uploadedName);
        done++;
        renderPdImages();
        next();
      }).catch(function(){
        failed++;
        next();
      });
    }
    next();
  }
  function openProd(id){
    var sel = document.getElementById("pdCat");
    if(sel && !sel.options.length){ CATS.forEach(function(c){ var o=document.createElement("option"); o.value=c; o.textContent=c; sel.appendChild(o); }); }
    var p = id ? (ADM.products||[]).find(function(x){ return x.id===id; }) : null;
    document.getElementById("prodTitle").textContent = p ? "Edit product" : "Add a product";
    document.getElementById("pdId").value = p ? p.id : "";
    document.getElementById("pdName").value = p ? p.name : "";
    document.getElementById("pdSize").value = p ? (p.size||"") : "";
    document.getElementById("pdPrice").value = p ? p.price : "";
    document.getElementById("pdMin").value = p ? (p.min||p.price) : "";
    document.getElementById("pdMax").value = p ? (p.max||p.price) : "";
    document.getElementById("pdTag").value = p ? (p.tag||"") : "";
    document.getElementById("pdCat").value = p ? p.cat : "Thobes";
    document.getElementById("pdStock").value = p ? p.stock : "10";
    document.getElementById("pdDesc").value = p ? p.desc : "";
    document.getElementById("pdSizes").value = p ? (p.sizes||[]).join(", ") : "52, 54, 56, 58";
    document.getElementById("pdFabric").value = p ? (p.fabric||"") : "";
    document.getElementById("pdFit").value = p ? (p.fit||"") : "";
    document.getElementById("pdCare").value = p ? (p.care||"") : "";
    pdImages = productImages(p);
    renderPdImages();
    loadPdImageLibrary();
    pdSetStatus("Up to 8 photos. The first is the cover — tap a photo to make it the cover.", "");
    document.getElementById("prodModal").classList.add("open");
  }
  function closeProd(){ document.getElementById("prodModal").classList.remove("open"); }
  document.getElementById("admProdAdd").addEventListener("click", function(){ openProd(null); });
  document.getElementById("pdCancel").addEventListener("click", closeProd);
  document.getElementById("prodModal").addEventListener("click", function(e){ if(e.target===this) closeProd(); });
  document.getElementById("pdImgPick").addEventListener("click", function(){ document.getElementById("pdImgFile").click(); });
  document.getElementById("pdImgFile").addEventListener("change", function(){
    uploadPdFiles(this.files);
    this.value = "";
  });
  document.getElementById("pdImgExisting").addEventListener("change", function(){
    var url = this.value; this.value = "";
    if(!url) return;
    if(pdImages.length >= 8){ pdSetStatus("You can add up to 8 photos per product.", "error"); return; }
    if(pdImages.indexOf(url) < 0) pdImages.push(url);
    renderPdImages();
    loadPdImageLibrary();
    pdSetStatus("Photo added. The first is the cover — tap another photo to change it.", "");
  });
  document.getElementById("pdImgGrid").addEventListener("click", function(e){
    var rm = e.target.closest("[data-pdremove]");
    if(rm){
      var removeUrl = rm.getAttribute("data-pdremove");
      pdImages = pdImages.filter(function(u){ return u !== removeUrl; });
      renderPdImages(); loadPdImageLibrary();
      pdSetStatus(pdImages.length ? "Photo removed. The first photo is the cover." : "Add at least one product photo.", "");
      return;
    }
    var th = e.target.closest("[data-pdimg]");
    if(!th) return;
    var url = th.getAttribute("data-pdimg");
    pdImages = [url].concat(pdImages.filter(function(u){ return u !== url; }));
    renderPdImages(); loadPdImageLibrary();
    pdSetStatus("Cover updated.", "");
  });
  document.getElementById("pdImgGrid").addEventListener("keydown", function(e){
    if((e.key === "Enter" || e.key === " ") && e.target.classList.contains("pd-thumb")){
      e.preventDefault(); e.target.click();
    }
  });
  document.getElementById("pdSave").addEventListener("click", function(){
    var name = document.getElementById("pdName").value.trim();
    var price = parseInt(document.getElementById("pdPrice").value,10)||0;
    var stock = parseInt(document.getElementById("pdStock").value,10);
    if(stock<0 || isNaN(stock)) stock = 0;
    var sizes = document.getElementById("pdSizes").value.split(",").map(function(s){ return s.trim(); }).filter(Boolean);
    if(!sizes.length) sizes = ["One size"];
    if(name.length<3){ notify("Give the product a name.","error"); return; }
    if(price<=0){ notify("Enter a price in DZD.","error"); return; }
    if(pdUploading){ notify("Wait for the photos to finish uploading.","error"); return; }
    if(!pdImages.length){ notify("Add at least one product photo.","error"); return; }
    var newId = document.getElementById("pdId").value.trim();
    if(!newId) newId = "product-" + Date.now().toString(36) + Math.random().toString(36).slice(2, 7);
    var body = {token:admToken, product:{
      id: newId,
      name: name,
      size: sizeLabelOf(sizes),
      price: price,
      min: price,
      max: price,
      tag: document.getElementById("pdTag").value.trim()||"New",
      cat: document.getElementById("pdCat").value,
      stock: stock,
      img: pdImages[0],
      images: pdImages.slice(0, 8),
      desc: document.getElementById("pdDesc").value.trim(),
      sizes: sizes,
      fabric: document.getElementById("pdFabric").value.trim(),
      fit: document.getElementById("pdFit").value.trim(),
      care: document.getElementById("pdCare").value.trim()
    }};
    admApi("/api/admin/product","POST",body).then(function(r){
      if(r && r.ok){
        closeProd(); notify("Product saved","success");
        adminLoad();
        syncStock().then(function(){ renderProducts(); }).catch(function(){});
      } else notify("Check the product details.","error");
    }).catch(function(){ notify("Could not save this product.","error"); });
  });

  /* Storefront / branding */
  var THEMES = {
    oud:      {preset:"oud", shade:"#f5f1e8", fg:"#201814", rose:"#8a6d3b", roseDeep:"#4a3b22", blue:"#2f4f43"},
    sand:     {preset:"sand", shade:"#f4ecdd", fg:"#2a2118", rose:"#d2a24c", roseDeep:"#9a6b23", blue:"#b3541e"},
    midnight: {preset:"midnight", shade:"#101418", fg:"#f2f0eb", rose:"#8fa3c7", roseDeep:"#5b718f", blue:"#0a84ff"},
    noir:     {preset:"noir", shade:"#ececec", fg:"#0a0a0a", rose:"#b5b5b5", roseDeep:"#5c5c5c", blue:"#0a0a0a"}
  };
  var PRESETS = [
    ["oud","Oud","Warm ivory"],
    ["sand","Sand","Soft beige"],
    ["midnight","Midnight","Deep navy"],
    ["noir","Noir","Gallery grey"]
  ];
  function fillAdminForms(){
    var s = ADM.settings || {};
    var sn = document.getElementById("admSiteName");
    if(sn) sn.value = s.siteName || "Noor";
    var brand = document.getElementById("admBrandName");
    if(brand) brand.textContent = s.siteName || "Noor";
    var pv = document.getElementById("admLogoPrev"), hint = document.getElementById("admLogoHint");
    if(pv && hint){
      if(s.logo){ pv.src = s.logo; pv.hidden=false; hint.textContent = "Custom logo in use"; }
      else { pv.hidden=true; pv.removeAttribute("src"); hint.textContent = "Moon icon in use"; }
    }
    var bn = s.banner || {};
    if(document.getElementById("admBanText")) document.getElementById("admBanText").value = bn.text || "";
    if(document.getElementById("admBanBg") && bn.bg) document.getElementById("admBanBg").value = bn.bg;
    if(document.getElementById("admBanFg") && bn.fg) document.getElementById("admBanFg").value = bn.fg;
    if(document.getElementById("admBanOn")) document.getElementById("admBanOn").checked = !!bn.active;
    var wa = document.getElementById("admWa");
    if(wa) wa.value = s.whatsapp || "";
    var soc = s.social || {};
    if(document.getElementById("admIg")) document.getElementById("admIg").value = soc.instagram || "";
    if(document.getElementById("admFb")) document.getElementById("admFb").value = soc.facebook || "";
    if(document.getElementById("admTt")) document.getElementById("admTt").value = soc.tiktok || "";
    var th = s.theme || {};
    var map = {shade:"admCShade", fg:"admCFg", rose:"admCRose", roseDeep:"admCRoseDeep", blue:"admCBlue"};
    Object.keys(map).forEach(function(k){ var el=document.getElementById(map[k]); if(el && th[k]) el.value = th[k]; });
    markPreset(th.preset);
    renderPresets();
    var free = document.getElementById("admFreeOver");
    if(free) free.textContent = money((ADM.shipping && ADM.shipping.freeOver) || 8000);
  }
  function renderAdmSettings(){
    var free = document.getElementById("admFreeOver");
    if(free) free.textContent = money((ADM.shipping && ADM.shipping.freeOver) || 8000);
  }
  function markPreset(name){
    Array.prototype.forEach.call(document.querySelectorAll("#admPresets .preset-btn"), function(b){
      b.classList.toggle("on", b.getAttribute("data-p")===name);
    });
  }
  function renderPresets(){
    var box = document.getElementById("admPresets");
    if(!box || box.children.length || !PRESETS.length) return;
    PRESETS.forEach(function(item){
      var name=item[0], label=item[1], desc=item[2], th=THEMES[name];
      if(!th) return;
      var b = document.createElement("button");
      b.type = "button"; b.className = "preset-btn"; b.setAttribute("data-p", name);
      b.innerHTML = '<i style="background:linear-gradient(135deg,'+th.rose+' 50%,'+th.blue+' 50%)"></i><span><b>'+label+'</b><small>'+desc+'</small></span>';
      b.addEventListener("click", function(){
        document.getElementById("admCShade").value = th.shade;
        document.getElementById("admCFg").value = th.fg;
        document.getElementById("admCRose").value = th.rose;
        document.getElementById("admCRoseDeep").value = th.roseDeep;
        document.getElementById("admCBlue").value = th.blue;
        applyTheme(th); markPreset(name);
      });
      box.appendChild(b);
    });
    if(SITE.theme) markPreset(SITE.theme.preset);
  }
  function curTheme(){
    return { preset:"custom",
      shade:document.getElementById("admCShade").value,
      fg:document.getElementById("admCFg").value,
      rose:document.getElementById("admCRose").value,
      roseDeep:document.getElementById("admCRoseDeep").value,
      blue:document.getElementById("admCBlue").value };
  }
  document.getElementById("admSiteSave").addEventListener("click", function(){
    var v = document.getElementById("admSiteName").value.trim()||"Noor";
    admApi("/api/admin/settings","POST",{token:admToken,siteName:v}).then(function(r){
      if(r && r.ok){ applyStoreSettings({siteName:v}); ADM.settings = Object.assign({}, ADM.settings, {siteName:v}); fillAdminForms(); notify("Store name saved","success"); }
      else notify("Could not save the name","error");
    }).catch(function(){ notify("Could not save the name","error"); });
  });
  document.getElementById("admLogoUp").addEventListener("click", function(){
    var f = document.getElementById("admLogoFile").files[0];
    if(!f){ notify("Choose an image first","error"); return; }
    if(f.size>1500000){ notify("Images must be under 1.5 MB","error"); return; }
    var rd = new FileReader();
    rd.onload = function(){
      admApi("/api/admin/logo","POST",{token:admToken,filename:f.name,data:rd.result}).then(function(r){
        if(r && r.ok){
          applyStoreSettings({logo:r.logo});
          ADM.settings = Object.assign({}, ADM.settings, {logo:r.logo});
          fillAdminForms(); notify("Logo updated","success");
        } else notify("That image could not be used","error");
      }).catch(function(){ notify("Upload failed","error"); });
    };
    rd.readAsDataURL(f);
  });
  document.getElementById("admLogoRm").addEventListener("click", function(){
    admApi("/api/admin/settings","POST",{token:admToken,logo:""}).then(function(){
      applyStoreSettings({logo:""});
      ADM.settings = Object.assign({}, ADM.settings, {logo:""});
      fillAdminForms(); notify("Logo removed","success");
    }).catch(function(){ notify("Could not remove the logo","error"); });
  });
  document.getElementById("admThemeSave").addEventListener("click", function(){
    var th = curTheme();
    admApi("/api/admin/settings","POST",{token:admToken,theme:th}).then(function(r){
      if(r && r.ok){ applyTheme(th); SITE.theme=th; markPreset("custom"); notify("Appearance saved","success"); }
      else notify("Could not save the appearance","error");
    }).catch(function(){ notify("Could not save the appearance","error"); });
  });
  document.getElementById("admBanSave").addEventListener("click", function(){
    var bn = { text:document.getElementById("admBanText").value.trim(),
      bg:document.getElementById("admBanBg").value, fg:document.getElementById("admBanFg").value,
      active:document.getElementById("admBanOn").checked };
    admApi("/api/admin/settings","POST",{token:admToken,banner:bn}).then(function(r){
      if(r && r.ok){ applyBanner(bn); SITE.banner=bn; ADM.settings=Object.assign({},ADM.settings,{banner:bn}); notify("Announcement saved","success"); }
      else notify("Could not save the announcement","error");
    }).catch(function(){ notify("Could not save the announcement","error"); });
  });

  /* Discounts */
  function promoList(){
    return (ADM.stats && ADM.stats.promos) ? ADM.stats.promos : (SITE.promos||[]);
  }
  function renderPromos(){
    var box = document.getElementById("admPromos");
    if(!box) return;
    var list = promoList();
    box.innerHTML = list.length ? list.map(function(p){
      return '<div class="adm-promo'+(p.active?"":" off")+'"><div class="adm-promo-main"><code>'+escapeHtml(p.code)+'</code><b>−'+p.pct+'%</b><span>'+escapeHtml(p.label||"")+'</span></div><div class="adm-promo-actions"><label class="adm-switch" title="Turn this code on or off"><input type="checkbox" data-pact="'+escapeHtml(p.code)+'"'+(p.active?" checked":"")+'><span></span></label><button type="button" class="adm-btn adm-btn-danger-ghost" data-pdel-promo="'+escapeHtml(p.code)+'">Delete</button></div></div>';
    }).join("") : '<div class="adm-empty">No discount codes yet. Add your first one on the right.</div>';
  }
  function savePromos(list){
    admApi("/api/admin/promos","POST",{token:admToken,promos:list}).then(function(r){
      if(r && r.ok){
        SITE.promos = (r.promos||[]).filter(function(p){ return p.active; });
        if(ADM.stats) ADM.stats.promos = r.promos;
        renderPromos(); notify("Discounts saved","success");
      } else notify("Add at least one valid code","error");
    }).catch(function(){ notify("Could not save discounts","error"); });
  }
  document.getElementById("admPromoAdd").addEventListener("click", function(){
    var code = document.getElementById("admPromoCode").value.trim().toUpperCase();
    var pct = parseInt(document.getElementById("admPromoPct").value,10);
    var label = document.getElementById("admPromoLabel").value.trim();
    if(!/^[A-Z0-9-]{2,20}$/.test(code)||!(pct>=1&&pct<=90)){ notify("Use 2–20 letters or numbers, and a discount from 1 to 90%.","error"); return; }
    var list = promoList().slice();
    var found = false;
    list = list.map(function(p){ if(p.code===code){ found=true; return {code:code,pct:pct,active:p.active,label:label}; } return p; });
    if(!found) list.push({code:code,pct:pct,active:true,label:label});
    document.getElementById("admPromoCode").value = "";
    document.getElementById("admPromoPct").value = "";
    document.getElementById("admPromoLabel").value = "";
    savePromos(list);
  });

  /* Settings */
  document.getElementById("admWaSave").addEventListener("click", function(){
    var v = document.getElementById("admWa").value.trim();
    var social = {
      instagram:(document.getElementById("admIg")||{}).value||"",
      facebook:(document.getElementById("admFb")||{}).value||"",
      tiktok:(document.getElementById("admTt")||{}).value||""
    };
    admApi("/api/admin/settings","POST",{token:admToken,whatsapp:v,social:social}).then(function(r){
      if(r && r.ok){
        WHATSAPP_NUMBER = r.whatsapp||"";
        SITE.social = r.social || social;
        ADM.settings = Object.assign({}, ADM.settings, {whatsapp:WHATSAPP_NUMBER, social:SITE.social});
        renderContact();
        notify("Contact details saved","success");
      } else notify("Check the number and links, then try again","error");
    }).catch(function(){ notify("Could not save contact details","error"); });
  });
  document.getElementById("admRefresh").addEventListener("click", function(){
    adminLoad(); notify("Refreshed","info");
  });
  function admLogout(){
    if(admToken) api("/api/admin/logout","POST",null,{"Authorization":"Bearer "+admToken}).catch(function(){});
    admSaveToken("");
    ["adminPin","adminCode","admSecPass","admSecCur","admSecCode","admSecOffCode"].forEach(function(id){ var el=document.getElementById(id); if(el) el.value=""; });
    var cw = document.getElementById("adminCodeWrap"); if(cw) cw.hidden = true;
    admSec = null;
    ADM = { orders:[], products:[], stats:null, settings:null, shipping:null, images:null };
    admLoaded = false;
    adminShowLogin();
  }
  ["adminOut","adminOut2"].forEach(function(id){
    var b = document.getElementById(id);
    if(b) b.addEventListener("click", admLogout);
  });
  document.getElementById("adminBackup").addEventListener("click", function(e){
    e.preventDefault();
    if(!admToken) return;
    fetch("/api/admin/backup",{method:"POST",headers:{"Content-Type":"application/json","Authorization":"Bearer "+admToken},body:JSON.stringify({token:admToken})}).then(function(r){
      if(!r.ok) throw new Error("backup");
      return r.blob();
    }).then(function(b){
      var a=document.createElement("a");
      a.href=URL.createObjectURL(b); a.download="noor-backup.json";
      document.body.appendChild(a); a.click();
      setTimeout(function(){ URL.revokeObjectURL(a.href); a.remove(); },500);
      notify("Backup downloaded","success");
    }).catch(function(){ notify("Backup failed","error"); });
  });
  function apiRaw(path, method, body, headers){
    var h = Object.assign({"Content-Type":"application/json"}, headers||{});
    return fetch(path, {method:method||"POST", headers:h, body:body?JSON.stringify(body):undefined}).then(function(r){
      return r.json().catch(function(){ return {}; }).then(function(j){ return {status:r.status, body:j||{}}; });
    });
  }
  document.getElementById("adminLoginForm").addEventListener("submit", function(e){
    e.preventDefault();
    var userEl = document.getElementById("adminUser"), pinEl = document.getElementById("adminPin");
    var codeEl = document.getElementById("adminCode"), codeWrap = document.getElementById("adminCodeWrap");
    var err = document.getElementById("adminLoginError"), btn = document.getElementById("adminGo");
    var user = userEl ? userEl.value.trim() : "", pass = pinEl.value;
    var code = codeEl ? codeEl.value.replace(/\D/g,"") : "";
    function showErr(m){ if(err){ err.textContent=m; err.hidden=false; } }
    if(!pass){ showErr("Enter your password to continue."); return; }
    if(err) err.hidden = true;
    if(btn){ btn.disabled=true; btn.textContent="Checking…"; }
    var body = {user:user, pass:pass}; if(code) body.code = code;
    apiRaw("/api/admin/login","POST",body).then(function(res){
      var r = res.body || {};
      if(res.status===200 && r.ok){
        admSaveToken(r.token);
        pinEl.value = ""; if(codeEl) codeEl.value = ""; if(codeWrap) codeWrap.hidden = true;
        setAdmTab(r.setup ? "settings" : "overview");
        renderAdmin();
        if(r.setup) notify("Create your username and password below","info");
        return;
      }
      if(r.error==="code-required"){ if(codeWrap) codeWrap.hidden=false; if(codeEl) codeEl.focus(); showErr("Enter the 6-digit code from your authenticator app."); return; }
      if(r.error==="code-wrong"){ if(codeWrap) codeWrap.hidden=false; if(codeEl){ codeEl.value=""; codeEl.focus(); } showErr("That code is not correct or was already used. Wait for the next code."); return; }
      if(res.status===429){ showErr(r.error==="locked" ? "Too many attempts. Wait "+Math.max(1,Math.ceil((r.retryAfter||60)/60))+" min, then try again." : "Too many attempts. Wait a minute, then try again."); return; }
      if(res.status===503){ showErr("Sign-in is turned off on the server. Set ADMIN_PIN in .env and restart the app."); return; }
      showErr("That username or password is not correct. Try again.");
    }).catch(function(){
      showErr("Can't reach the server. Check your connection and try again.");
    }).then(function(){
      if(btn){ btn.disabled=false; btn.textContent="Enter"; }
    });
  });
  /* ----- Admin: sign-in & security card ----- */
  var admSec = null;
  function secEl(id){ return document.getElementById(id); }
  function admSecRender(){
    var s = admSec || {};
    if(!secEl("admSecCard")) return;
    secEl("admSecStatus").textContent = s.account ? ("Signed in as "+s.user+" · 2-step verification "+(s.totp?"on":"off")+".") : "No admin account yet — you used the temporary PIN.";
    secEl("admSecSetup").hidden = !!s.account;
    secEl("admSecCurLabel").textContent = s.account ? "Current password" : "Current PIN (ADMIN_PIN in .env)";
    if(s.account && !secEl("admSecUser").value) secEl("admSecUser").value = s.user || "";
    secEl("admSecSave").textContent = s.account ? "Change sign-in details" : "Create admin account";
    secEl("admSec2fa").hidden = !s.account;
    secEl("admSec2faStatus").textContent = s.totp ? "On — sign-in asks for a code from your phone." : "Off";
    secEl("admSec2faStart").hidden = !!s.totp;
    secEl("admSec2faOff").hidden = !s.totp;
    if(s.totp) secEl("admSec2faSetup").hidden = true;
    secEl("admSecMail").textContent = s.mail ? "On" : "Off — email not set up";
    secEl("admSecAdminMail").textContent = (s.mail && s.adminEmail) ? "On" : "Off";
    secEl("admSecBackupMail").textContent = (s.mail && s.backupEmail) ? "On" : "Off";
  }
  function admSecLoad(){
    if(!admToken) return;
    return admApi("/api/admin/security","POST",{}).then(function(r){ admSec = r; admSecRender(); }).catch(function(){});
  }
  var SEC_ERR = {
    "current-wrong":"The current password (or PIN) is not correct.",
    "user-invalid":"Use 3–40 letters, numbers, dots, dashes or underscores for the username.",
    "pass-weak":"Use at least 10 characters, different from the username.",
    "code-wrong":"That code is not correct. Check the app and your current password.",
    "start-again":"The setup expired. Start again.",
    "account-first":"Create your username and password first.",
    "locked":"Too many attempts. Wait 15 minutes, then try again."
  };
  function admSecPost(path, body){
    return apiRaw(path, "POST", body, {"Authorization":"Bearer "+admToken}).then(function(res){
      if(res.status===401){ admSaveToken(""); adminShowLogin(); throw new Error("auth"); }
      if(res.status!==200 || !res.body.ok){ notify(SEC_ERR[res.body.error] || "Something went wrong. Try again.","error"); throw new Error("sec"); }
      return res.body;
    });
  }
  function ensureQr(cb){
    if(window.qrcode){ cb(); return; }
    var sc = document.createElement("script");
    sc.src = "qrcode.js?v=1.4.4";
    sc.onload = cb;
    sc.onerror = function(){ var q=secEl("admSecQr"); if(q) q.textContent = "QR code unavailable — type the key below."; };
    document.head.appendChild(sc);
  }
  if(secEl("admSecSave")) secEl("admSecSave").addEventListener("click", function(){
    var btn = this;
    btn.disabled = true;
    admSecPost("/api/admin/security/account", {user:secEl("admSecUser").value.trim(), pass:secEl("admSecPass").value, current:secEl("admSecCur").value}).then(function(){
      secEl("admSecPass").value = ""; secEl("admSecCur").value = "";
      notify("Saved. Sign in with your username and password from now on.","success");
      return admSecLoad();
    }).catch(function(){}).then(function(){ btn.disabled = false; });
  });
  if(secEl("admSec2faStart")) secEl("admSec2faStart").addEventListener("click", function(){
    admSecPost("/api/admin/security/totp-start", {}).then(function(r){
      secEl("admSecSecret").textContent = r.secret.replace(/(.{4})/g,"$1 ").trim();
      secEl("admSecQr").textContent = "";
      ensureQr(function(){
        var q = window.qrcode(0, "M"); q.addData(r.uri); q.make();
        secEl("admSecQr").innerHTML = q.createSvgTag(4, 2);
      });
      secEl("admSec2faSetup").hidden = false; secEl("admSec2faStart").hidden = true;
      secEl("admSecCode").focus();
    }).catch(function(){});
  });
  if(secEl("admSec2faConfirm")) secEl("admSec2faConfirm").addEventListener("click", function(){
    admSecPost("/api/admin/security/totp-enable", {current:secEl("admSecCur").value, code:secEl("admSecCode").value.replace(/\D/g,"")}).then(function(){
      secEl("admSecCode").value = ""; secEl("admSecCur").value = ""; secEl("admSecQr").textContent = ""; secEl("admSecSecret").textContent = "";
      notify("2-step verification is on.","success");
      return admSecLoad();
    }).catch(function(){});
  });
  if(secEl("admSec2faDisable")) secEl("admSec2faDisable").addEventListener("click", function(){
    admSecPost("/api/admin/security/totp-disable", {current:secEl("admSecCur").value, code:secEl("admSecOffCode").value.replace(/\D/g,"")}).then(function(){
      secEl("admSecOffCode").value = ""; secEl("admSecCur").value = "";
      notify("2-step verification is off.","info");
      return admSecLoad();
    }).catch(function(){});
  });
  (function(){
    var os=document.getElementById("admOrderSearch");
    if(os) os.addEventListener("input", function(){ admOrderQ = os.value.trim(); renderAdmOrders(); });
    var ps=document.getElementById("admProductSearch");
    if(ps) ps.addEventListener("input", function(){ admProductQ = ps.value.trim(); renderAdmProducts(); });
    var panel=document.getElementById("adminPanel");
    if(!panel) return;
    panel.addEventListener("click", function(e){
      var tab=e.target.closest("[data-adm-tab]");
      if(tab){ setAdmTab(tab.getAttribute("data-adm-tab")); return; }
      var goto=e.target.closest("[data-goto]");
      if(goto){ setAdmTab(goto.getAttribute("data-goto")); return; }
      var filt=e.target.closest("#admOrderFilters [data-status]");
      if(filt){
        admOrderFilter = filt.getAttribute("data-status");
        Array.prototype.forEach.call(document.querySelectorAll("#admOrderFilters [data-status]"), function(b){ b.classList.toggle("on", b===filt); });
        renderAdmOrders(); return;
      }
      var ost=e.target.closest("[data-ostatus]");
      if(ost){ admUpdateStatus(ost.getAttribute("data-onum"), ost.getAttribute("data-ostatus")); return; }
      var mini=e.target.closest("[data-order]");
      if(mini){
        admOrderQ = mini.getAttribute("data-order");
        admOrderFilter = "all";
        var si=document.getElementById("admOrderSearch"); if(si) si.value = admOrderQ;
        Array.prototype.forEach.call(document.querySelectorAll("#admOrderFilters [data-status]"), function(b){ b.classList.toggle("on", b.getAttribute("data-status")==="all"); });
        setAdmTab("orders"); return;
      }
      var pc=e.target.closest("[data-pcheck]");
      if(pc){
        var checkId = pc.getAttribute("data-pcheck");
        if(pc.checked) admSelProducts[checkId] = true; else delete admSelProducts[checkId];
        var checkCard = pc.closest(".adm-prod");
        if(checkCard) checkCard.classList.toggle("selected", !!admSelProducts[checkId]);
        renderAdmBulkBar(); return;
      }
      if(e.target.id === "admProdSelectAll"){
        var selectAll = e.target.checked;
        admSelProducts = {};
        if(selectAll) admVisibleProducts.forEach(function(id){ admSelProducts[id] = true; });
        renderAdmProducts(); return;
      }
      if(e.target.id === "admProdBulkClear"){ clearAdmProductSelection(); return; }
      if(e.target.id === "admProdBulkDelete"){
        var ids = Object.keys(admSelProducts).filter(function(id){ return admSelProducts[id]; });
        if(!ids.length) return;
        if(!confirm("Delete "+ids.length+" selected product"+(ids.length===1?"":"s")+"? They will disappear from the store.")) return;
        var bulkBtn = e.target; bulkBtn.disabled = true;
        admApi("/api/admin/products/delete","POST",{token:admToken,ids:ids}).then(function(r){
          if(r && r.ok){
            admSelProducts = {};
            notify((r.deleted || ids.length)+" product"+(ids.length===1?"":"s")+" deleted","success");
            adminLoad();
            syncStock().then(function(){ renderProducts(); }).catch(function(){});
          } else { bulkBtn.disabled = false; notify("Could not delete the selected products","error"); }
        }).catch(function(){ bulkBtn.disabled = false; notify("Could not delete the selected products","error"); });
        return;
      }
      var pe=e.target.closest("[data-pedit]");
      if(pe){ openProd(pe.getAttribute("data-pedit")); return; }
      var pd=e.target.closest("[data-pdel]");
      if(pd){
        var pid = pd.getAttribute("data-pdel");
        if(!confirm("Delete this product? It will disappear from the store.")) return;
        delete admSelProducts[pid];
        admApi("/api/admin/product/delete","POST",{token:admToken,id:pid}).then(function(){
          notify("Product deleted","success");
          adminLoad();
          syncStock().then(function(){ renderProducts(); }).catch(function(){});
        }).catch(function(){ notify("Could not delete this product","error"); });
        return;
      }
      var pa=e.target.closest("[data-pact]");
      if(pa){
        var code = pa.getAttribute("data-pact");
        var list = promoList().slice().map(function(p){ if(p.code===code) p.active = !p.active; return p; });
        savePromos(list); return;
      }
      var pdel=e.target.closest("[data-pdel-promo]");
      if(pdel){
        var code2 = pdel.getAttribute("data-pdel-promo");
        var list2 = promoList().slice();
        if(list2.length<=1){ notify("Keep at least one discount code","error"); return; }
        if(!confirm("Delete the code "+code2+"?")) return;
        savePromos(list2.filter(function(p){ return p.code!==code2; }));
        return;
      }
    });
  })();

  /* Boot never depends on the network: the fallback catalog is baked in, the
     first paint happens immediately, and a later stock sync only re-renders. */
  var booted=false;
  function bootGo(){
    if(booted) return;
    booted=true;
    try{ fetchSettings(); }catch(e){}
    try{ applyLang(lang); }catch(e){}
    try{ handleRoute(); }catch(e){}
    try{ renderContact(); }catch(e){}
    clipInit();
    if(window.pageYOffset>0) requestAnimationFrame(readScroll);
  }
  bootGo();
  syncStock().then(function(){
    if(booted){ renderProducts(); renderCart(); updateMiniAdds(); }
  }, function(){});
})();
