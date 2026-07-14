// Glint website localization: English, Greek, Albanian. No build step - plain
// data + a tiny apply pass over [data-i18n] / [data-i18n-html] attributes.
// Language is detected from the browser and can be switched in the menu bar.

const GLINT_SITE_I18N = {
  en: {
    title: "Glint - a glint of Git in your menu bar",
    navFeatures: "Features",
    navThemes: "Themes",
    navHow: "How it works",
    navDownload: "Download",
    heroEyebrow: "Menu-bar Git · macOS",
    heroH1: `Your repo, <em>one click</em> from the menu&nbsp;bar.`,
    heroLede: `Glint is an ultralightweight Git client that hangs from your menu bar. Click the icon, the glass drops down - stage, commit, sync - then it's gone. Built on <strong>Tauri&nbsp;+&nbsp;Rust</strong>, not Electron.`,
    ctaDownload: "Download for macOS",
    ctaGithub: "View on GitHub",
    statBundle: "app bundle",
    statMemory: "memory",
    statChromium: "Chromium",
    featEyebrow: "Why Glint",
    featH2: "Small, glassy, and out of your way.",
    feat1Title: "Tiny",
    feat1Body: "The system webview (WebKit) plus a Rust core - no bundled Chromium riding along. Megabytes, not hundreds of them.",
    feat2Title: "Glassy",
    feat2Body: "Real macOS vibrancy sits behind a transparent webview - genuine liquid glass, not a flat translucent fill. Themes tint the glass.",
    feat3Title: "Menu-bar native",
    feat3Body: "A tray-positioned panel that toggles on click and dismisses on blur. Glanceable - identity, sync state, changes, and commit in one dropdown.",
    feat4Title: "Yours",
    feat4Body: "Every theme is a handful of CSS tokens - an accent, a tint, an ink pair. Drop one in and it appears as a swatch. No redesign required.",
    themesEyebrow: "Themeable by token",
    themesH2: "Same glass. Swapped tokens.",
    themesLede: "Light or dark, the material never changes - only the tint and accent do. Midnight is the proof: the tint flips to a dark translucent and the ink inverts, but the radii, blur, and shadow are untouched.",
    themesHint: "Pick a theme - the panel and this whole site retint live.",
    howEyebrow: "Under the glass",
    howH2: "A Rust core and the system webview.",
    howText: "The frontend is plain HTML, CSS, and JavaScript - no framework - rendered by the OS webview. The core is Rust: it owns the tray, the transparent window, native vibrancy, and Git.",
    howLi1: `<strong>Tray &amp; window</strong> - a borderless, always-on-top panel positioned under the menu-bar icon.`,
    howLi2: `<strong>Vibrancy</strong> - <code>NSVisualEffectView</code> behind the transparent webview for real glass.`,
    howLi3: `<strong>Git</strong> - native push, pull, and PR status in-process via <code>git2</code>, using your SSH keys or a token.`,
    dlEyebrow: "Get Glint",
    dlH2: "Install in one line.",
    dlLede: `Glint is early - <code>v0.1.0</code>, macOS, open source. Grab the cask or build it from source in a minute.`,
    copyLabel: "Copy",
    copiedLabel: "Copied",
    dlDmg: "Download .dmg",
    dlSource: "Build from source",
    dlNote: `Requires macOS 12+. Apple silicon &amp; Intel.`,
    footTagline: "A glint of Git in your menu bar.",
    footGithub: "GitHub",
    footDesign: "Design language",
    footIssues: "Issues",
    footDownload: "Download",
    footLegal: `Built by <a href="https://peterdsp.dev">peterdsp</a> · Tauri + Rust · <span id="year"></span>`,
  },

  el: {
    title: "Glint - μια λάμψη Git στη γραμμή μενού σου",
    navFeatures: "Δυνατότητες",
    navThemes: "Θέματα",
    navHow: "Πώς λειτουργεί",
    navDownload: "Λήψη",
    heroEyebrow: "Git στη γραμμή μενού · macOS",
    heroH1: `Το repo σου, <em>με ένα κλικ</em> από τη γραμμή&nbsp;μενού.`,
    heroLede: `Το Glint είναι ένας εξαιρετικά ελαφρύς Git client που κρέμεται από τη γραμμή μενού σου. Κάνε κλικ στο εικονίδιο, το γυαλί κατεβαίνει - stage, commit, sync - και μετά χάνεται. Φτιαγμένο σε <strong>Tauri&nbsp;+&nbsp;Rust</strong>, όχι Electron.`,
    ctaDownload: "Λήψη για macOS",
    ctaGithub: "Δες το στο GitHub",
    statBundle: "μέγεθος εφαρμογής",
    statMemory: "μνήμη",
    statChromium: "Chromium",
    featEyebrow: "Γιατί Glint",
    featH2: "Μικρό, γυάλινο, και μακριά από τα πόδια σου.",
    feat1Title: "Μικροσκοπικό",
    feat1Body: "Το webview του συστήματος (WebKit) μαζί με έναν πυρήνα Rust - χωρίς πακεταρισμένο Chromium από πίσω. Megabytes, όχι εκατοντάδες.",
    feat2Title: "Γυάλινο",
    feat2Body: "Πραγματικό macOS vibrancy πίσω από ένα διάφανο webview - αυθεντικό liquid glass, όχι ένα επίπεδο ημιδιάφανο γέμισμα. Τα θέματα χρωματίζουν το γυαλί.",
    feat3Title: "Εγγενές στη γραμμή μενού",
    feat3Body: "Ένα πάνελ τοποθετημένο στη γραμμή που ανοίγει με κλικ και κλείνει όταν χάσει την εστίαση. Με μια ματιά - ταυτότητα, κατάσταση sync, αλλαγές και commit σε ένα dropdown.",
    feat4Title: "Δικό σου",
    feat4Body: "Κάθε θέμα είναι λίγα CSS tokens - ένα accent, ένα tint, ένα ζευγάρι ink. Ρίξε ένα μέσα και εμφανίζεται ως δείγμα. Χωρίς επανασχεδίαση.",
    themesEyebrow: "Θέματα με tokens",
    themesH2: "Ίδιο γυαλί. Αλλαγμένα tokens.",
    themesLede: "Φωτεινό ή σκούρο, το υλικό δεν αλλάζει ποτέ - μόνο το tint και το accent. Το Midnight είναι η απόδειξη: το tint γίνεται σκούρο ημιδιάφανο και το ink αντιστρέφεται, αλλά οι καμπύλες, το blur και η σκιά μένουν ίδια.",
    themesHint: "Διάλεξε ένα θέμα - το πάνελ και όλο το site ξαναχρωματίζονται ζωντανά.",
    howEyebrow: "Κάτω από το γυαλί",
    howH2: "Ένας πυρήνας Rust και το webview του συστήματος.",
    howText: "Το frontend είναι απλό HTML, CSS και JavaScript - χωρίς framework - που το αποδίδει το webview του λειτουργικού. Ο πυρήνας είναι Rust: κρατάει το tray, το διάφανο παράθυρο, το native vibrancy και το Git.",
    howLi1: `<strong>Tray &amp; παράθυρο</strong> - ένα πάνελ χωρίς περίγραμμα, πάντα από πάνω, τοποθετημένο κάτω από το εικονίδιο της γραμμής μενού.`,
    howLi2: `<strong>Vibrancy</strong> - <code>NSVisualEffectView</code> πίσω από το διάφανο webview για πραγματικό γυαλί.`,
    howLi3: `<strong>Git</strong> - εγγενή push, pull και κατάσταση PR εντός διεργασίας μέσω <code>git2</code>, με τα SSH κλειδιά σου ή ένα token.`,
    dlEyebrow: "Απόκτησε το Glint",
    dlH2: "Εγκατάσταση σε μία γραμμή.",
    dlLede: `Το Glint είναι νωρίς - <code>v0.1.0</code>, macOS, ανοιχτού κώδικα. Πάρε το cask ή φτιάξ' το από τον κώδικα σε ένα λεπτό.`,
    copyLabel: "Αντιγραφή",
    copiedLabel: "Αντιγράφηκε",
    dlDmg: "Λήψη .dmg",
    dlSource: "Φτιάξ' το από τον κώδικα",
    dlNote: `Απαιτεί macOS 12+. Apple silicon &amp; Intel.`,
    footTagline: "Μια λάμψη Git στη γραμμή μενού σου.",
    footGithub: "GitHub",
    footDesign: "Γλώσσα σχεδίασης",
    footIssues: "Ζητήματα",
    footDownload: "Λήψη",
    footLegal: `Φτιαγμένο από τον <a href="https://peterdsp.dev">peterdsp</a> · Tauri + Rust · <span id="year"></span>`,
  },

  sq: {
    title: "Glint - një shkëlqim Git në shiritin e menysë",
    navFeatures: "Veçoritë",
    navThemes: "Temat",
    navHow: "Si funksionon",
    navDownload: "Shkarko",
    heroEyebrow: "Git në shiritin e menysë · macOS",
    heroH1: `Depoja jote, <em>me një klik</em> nga shiriti i&nbsp;menysë.`,
    heroLede: `Glint është një klient Git tejet i lehtë që rri varur në shiritin e menysë. Kliko ikonën, xhami zbret - stage, commit, sync - pastaj zhduket. Ndërtuar me <strong>Tauri&nbsp;+&nbsp;Rust</strong>, jo Electron.`,
    ctaDownload: "Shkarko për macOS",
    ctaGithub: "Shikoje në GitHub",
    statBundle: "madhësia e aplikacionit",
    statMemory: "memoria",
    statChromium: "Chromium",
    featEyebrow: "Pse Glint",
    featH2: "I vogël, prej xhami, dhe jashtë rrugës sate.",
    feat1Title: "I vogël",
    feat1Body: "Webview-i i sistemit (WebKit) plus një bërthamë Rust - pa Chromium të paketuar bashkë. Megabajt, jo qindra.",
    feat2Title: "Prej xhami",
    feat2Body: "Vibrancy i vërtetë i macOS-it qëndron pas një webview-i transparent - liquid glass i mirëfilltë, jo një mbushje e sheshtë gjysmë-transparente. Temat e ngjyrosin xhamin.",
    feat3Title: "Vendas në shiritin e menysë",
    feat3Body: "Një panel i vendosur në shirit që hapet me klik dhe mbyllet kur humb fokusin. Me një shikim - identiteti, gjendja e sync, ndryshimet dhe commit në një dropdown.",
    feat4Title: "Yti",
    feat4Body: "Çdo temë është pak CSS tokens - një accent, një tint, një çift ink. Hidh një brenda dhe shfaqet si mostër. Pa rindërtim.",
    themesEyebrow: "E personalizueshme me tokens",
    themesH2: "I njëjti xham. Tokens të ndërruar.",
    themesLede: "I çelët apo i errët, materiali s'ndryshon kurrë - vetëm tint dhe accent. Midnight është prova: tint kthehet në të errët gjysmë-transparent dhe ink përmbyset, por rrezet, blur dhe hija mbeten të paprekura.",
    themesHint: "Zgjidh një temë - paneli dhe gjithë faqja ringjyrosen drejtpërdrejt.",
    howEyebrow: "Nën xham",
    howH2: "Një bërthamë Rust dhe webview-i i sistemit.",
    howText: "Pjesa e përparme është HTML, CSS dhe JavaScript i thjeshtë - pa framework - e vizatuar nga webview-i i sistemit. Bërthama është Rust: mban tray-in, dritaren transparente, vibrancy-n vendas dhe Git-in.",
    howLi1: `<strong>Tray &amp; dritare</strong> - një panel pa kornizë, gjithmonë sipër, i vendosur nën ikonën e shiritit të menysë.`,
    howLi2: `<strong>Vibrancy</strong> - <code>NSVisualEffectView</code> pas webview-it transparent për xham të vërtetë.`,
    howLi3: `<strong>Git</strong> - push, pull dhe gjendje PR vendase brenda procesit me <code>git2</code>, duke përdorur çelësat SSH ose një token.`,
    dlEyebrow: "Merr Glint",
    dlH2: "Instalo në një rresht.",
    dlLede: `Glint është në fillimet - <code>v0.1.0</code>, macOS, me kod të hapur. Merr cask-un ose ndërtoje nga kodi për një minutë.`,
    copyLabel: "Kopjo",
    copiedLabel: "U kopjua",
    dlDmg: "Shkarko .dmg",
    dlSource: "Ndërtoje nga kodi",
    dlNote: `Kërkon macOS 12+. Apple silicon &amp; Intel.`,
    footTagline: "Një shkëlqim Git në shiritin e menysë.",
    footGithub: "GitHub",
    footDesign: "Gjuha e dizajnit",
    footIssues: "Problemet",
    footDownload: "Shkarko",
    footLegal: `Ndërtuar nga <a href="https://peterdsp.dev">peterdsp</a> · Tauri + Rust · <span id="year"></span>`,
  },
};

function detectSiteLang() {
  try {
    const s = localStorage.getItem("glint.site.lang");
    if (s && GLINT_SITE_I18N[s]) return s;
  } catch (e) {}
  const n = (navigator.language || "en").toLowerCase();
  if (n.startsWith("el")) return "el";
  if (n.startsWith("sq")) return "sq";
  return "en";
}

function applySiteI18n(lang) {
  const dict = GLINT_SITE_I18N[lang] || GLINT_SITE_I18N.en;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const v = dict[el.dataset.i18n];
    if (v != null) el.textContent = v;
  });
  document.querySelectorAll("[data-i18n-html]").forEach((el) => {
    const v = dict[el.dataset.i18nHtml];
    if (v != null) el.innerHTML = v;
  });
  document.documentElement.lang = lang;
  if (dict.title) document.title = dict.title;
  document.querySelectorAll(".lang-btn").forEach((b) => {
    const on = b.dataset.lang === lang;
    b.classList.toggle("active", on);
    b.setAttribute("aria-pressed", String(on));
  });
  try {
    localStorage.setItem("glint.site.lang", lang);
  } catch (e) {}
  window.__glintLang = lang;
  const y = document.getElementById("year");
  if (y) y.textContent = String(new Date().getFullYear());
  document.dispatchEvent(new CustomEvent("glint:lang", { detail: lang }));
}

// Lookup helper for JS-set strings (e.g. the copy button state).
window.GLINT_T = (k) =>
  (GLINT_SITE_I18N[window.__glintLang || "en"] || GLINT_SITE_I18N.en)[k] || k;

document.querySelectorAll(".lang-btn").forEach((b) =>
  b.addEventListener("click", () => applySiteI18n(b.dataset.lang))
);
applySiteI18n(detectSiteLang());
