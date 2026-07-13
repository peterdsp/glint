// Localization for Glint - English, Greek (el), Albanian (sq).
//
// Strings are keyed; `t(key, vars)` substitutes {placeholders}. Static markup
// carries data-i18n (textContent) or data-i18n-ph (placeholder); dynamic
// strings call t() directly. Locale is saved in localStorage, else taken from
// the OS via navigator.language, else English.

window.GLINT_I18N = {
  en: {
    settings: "Settings",
    theme: "Theme",
    language: "Language",
    license: "License",
    updates: "Updates",
    toPull: "to pull",
    toPush: "to push",
    changedFiles: "{n} changed files",
    changedFile: "1 changed file",
    summary: "Summary",
    descriptionOptional: "Description (optional)",
    commitTo: "Commit to {branch}",
    trialBanner: "Trial - {n} days left · Get a license",
    trialBanner1: "Trial - 1 day left · Get a license",
    trialEndedTitle: "Your Glint trial has ended",
    trialEndedSub: "Keep Glint for a one-time $4.99 on Ko-fi. Already bought? Paste your key below.",
    getLicenseKofi: "Get a license on Ko-fi",
    orEnterKey: "or enter your license key",
    activate: "Activate",
    activateKey: "Activate key",
    pasteKey: "Paste license key",
    freeTrialLeft: "Free trial - {n} days left",
    freeTrialLeft1: "Free trial - 1 day left",
    licensedTo: "Licensed to {email}",
    licensedThanks: "Licensed - thank you!",
    trialEndedShort: "Trial ended - enter a license key",
    unlockedDev: "Unlocked (development)",
    updatesAuto: "Updates install automatically in the background.",
    checkNow: "Check now",
    checkingUpdates: "Checking for updates...",
    upToDateVersion: "You're on the latest version.",
    updateFound: "Update found - installing...",
    updateUnavailable: "Update check unavailable right now.",
    fetching: "Fetching...",
    pulling: "Pulling...",
    pushing: "Pushing...",
    pulled: "Pulled",
    pushed: "Pushed",
    upToDate: "Up to date",
    syncCounts: "{behind} to pull · {ahead} to push",
    connectRepo: "Connect a repo first - the selector up top",
    connectRepoDiff: "Connect a repo to see diffs",
    activating: "Checking...",
    activated: "Activated - thank you!",
    keyInvalid: "That key isn't valid.",
    switchRepoPrompt: "Path to a Git repository:",
    openDiff: "Open diff",
    noChanges: "No changes in this file.",
    binaryFile: "Binary file - no line diff.",
    loadingDiff: "Loading diff...",
  },

  el: {
    settings: "Ρυθμίσεις",
    theme: "Θέμα",
    language: "Γλώσσα",
    license: "Άδεια",
    updates: "Ενημερώσεις",
    toPull: "για λήψη",
    toPush: "για αποστολή",
    changedFiles: "{n} αλλαγμένα αρχεία",
    changedFile: "1 αλλαγμένο αρχείο",
    summary: "Σύνοψη",
    descriptionOptional: "Περιγραφή (προαιρετικό)",
    commitTo: "Υποβολή στο {branch}",
    trialBanner: "Δοκιμή - {n} ημέρες ακόμη · Αποκτήστε άδεια",
    trialBanner1: "Δοκιμή - 1 ημέρα ακόμη · Αποκτήστε άδεια",
    trialEndedTitle: "Η δοκιμή του Glint έληξε",
    trialEndedSub: "Κρατήστε το Glint με μια εφάπαξ πληρωμή $4.99 στο Ko-fi. Το αγοράσατε ήδη; Επικολλήστε το κλειδί σας παρακάτω.",
    getLicenseKofi: "Αποκτήστε άδεια στο Ko-fi",
    orEnterKey: "ή εισάγετε το κλειδί άδειας",
    activate: "Ενεργοποίηση",
    activateKey: "Ενεργοποίηση κλειδιού",
    pasteKey: "Επικολλήστε το κλειδί άδειας",
    freeTrialLeft: "Δωρεάν δοκιμή - {n} ημέρες ακόμη",
    freeTrialLeft1: "Δωρεάν δοκιμή - 1 ημέρα ακόμη",
    licensedTo: "Άδεια για {email}",
    licensedThanks: "Άδεια ενεργή - ευχαριστούμε!",
    trialEndedShort: "Η δοκιμή έληξε - εισάγετε κλειδί άδειας",
    unlockedDev: "Ξεκλείδωτο (ανάπτυξη)",
    updatesAuto: "Οι ενημερώσεις εγκαθίστανται αυτόματα στο παρασκήνιο.",
    checkNow: "Έλεγχος τώρα",
    checkingUpdates: "Έλεγχος για ενημερώσεις...",
    upToDateVersion: "Έχετε την πιο πρόσφατη έκδοση.",
    updateFound: "Βρέθηκε ενημέρωση - εγκατάσταση...",
    updateUnavailable: "Ο έλεγχος ενημερώσεων δεν είναι διαθέσιμος τώρα.",
    fetching: "Λήψη...",
    pulling: "Λήψη αλλαγών...",
    pushing: "Αποστολή...",
    pulled: "Ολοκληρώθηκε η λήψη",
    pushed: "Ολοκληρώθηκε η αποστολή",
    upToDate: "Ενημερωμένο",
    syncCounts: "{behind} για λήψη · {ahead} για αποστολή",
    connectRepo: "Συνδέστε πρώτα ένα αποθετήριο - ο επιλογέας επάνω",
    connectRepoDiff: "Συνδέστε ένα αποθετήριο για να δείτε διαφορές",
    activating: "Έλεγχος...",
    activated: "Ενεργοποιήθηκε - ευχαριστούμε!",
    keyInvalid: "Το κλειδί δεν είναι έγκυρο.",
    switchRepoPrompt: "Διαδρομή προς αποθετήριο Git:",
    openDiff: "Άνοιγμα διαφορών",
    noChanges: "Καμία αλλαγή σε αυτό το αρχείο.",
    binaryFile: "Δυαδικό αρχείο - χωρίς διαφορές γραμμών.",
    loadingDiff: "Φόρτωση διαφορών...",
  },

  sq: {
    settings: "Cilësimet",
    theme: "Tema",
    language: "Gjuha",
    license: "Licenca",
    updates: "Përditësimet",
    toPull: "për të marrë",
    toPush: "për të dërguar",
    changedFiles: "{n} skedarë të ndryshuar",
    changedFile: "1 skedar i ndryshuar",
    summary: "Përmbledhje",
    descriptionOptional: "Përshkrimi (opsional)",
    commitTo: "Kryej te {branch}",
    trialBanner: "Provë - edhe {n} ditë · Merr një licencë",
    trialBanner1: "Provë - edhe 1 ditë · Merr një licencë",
    trialEndedTitle: "Prova e Glint përfundoi",
    trialEndedSub: "Mbaje Glint me një pagesë të vetme $4.99 në Ko-fi. E ke blerë tashmë? Ngjit çelësin tënd më poshtë.",
    getLicenseKofi: "Merr një licencë në Ko-fi",
    orEnterKey: "ose vendos çelësin e licencës",
    activate: "Aktivizo",
    activateKey: "Aktivizo çelësin",
    pasteKey: "Ngjit çelësin e licencës",
    freeTrialLeft: "Provë falas - edhe {n} ditë",
    freeTrialLeft1: "Provë falas - edhe 1 ditë",
    licensedTo: "Licencuar për {email}",
    licensedThanks: "I licencuar - faleminderit!",
    trialEndedShort: "Prova përfundoi - vendos një çelës licence",
    unlockedDev: "I shkyçur (zhvillim)",
    updatesAuto: "Përditësimet instalohen automatikisht në sfond.",
    checkNow: "Kontrollo tani",
    checkingUpdates: "Duke kontrolluar për përditësime...",
    upToDateVersion: "Ke versionin më të fundit.",
    updateFound: "U gjet përditësim - duke instaluar...",
    updateUnavailable: "Kontrolli i përditësimeve s'është i disponueshëm tani.",
    fetching: "Duke marrë...",
    pulling: "Duke tërhequr...",
    pushing: "Duke dërguar...",
    pulled: "U tërhoq",
    pushed: "U dërgua",
    upToDate: "I përditësuar",
    syncCounts: "{behind} për të marrë · {ahead} për të dërguar",
    connectRepo: "Lidh një depo së pari - zgjedhësi lart",
    connectRepoDiff: "Lidh një depo për të parë ndryshimet",
    activating: "Duke kontrolluar...",
    activated: "U aktivizua - faleminderit!",
    keyInvalid: "Ky çelës nuk është i vlefshëm.",
    switchRepoPrompt: "Rruga drejt një depoje Git:",
    openDiff: "Hap ndryshimet",
    noChanges: "Asnjë ndryshim në këtë skedar.",
    binaryFile: "Skedar binar - pa ndryshime rreshtash.",
    loadingDiff: "Duke ngarkuar ndryshimet...",
  },
};

window.GLINT_LOCALES = [
  { code: "en", label: "English" },
  { code: "el", label: "Ελληνικά" },
  { code: "sq", label: "Shqip" },
];

let _locale = "en";

function detectLocale() {
  try {
    const saved = localStorage.getItem("glint.locale");
    if (saved && window.GLINT_I18N[saved]) return saved;
  } catch (e) {}
  const nav = (navigator.language || "en").slice(0, 2).toLowerCase();
  return window.GLINT_I18N[nav] ? nav : "en";
}

window.currentLocale = function () {
  return _locale;
};

// Translate a key with {var} substitution, falling back to English then the key.
window.t = function (key, vars) {
  const dict = window.GLINT_I18N[_locale] || window.GLINT_I18N.en;
  let s = dict[key] != null ? dict[key] : window.GLINT_I18N.en[key];
  if (s == null) return key;
  if (vars) {
    for (const k in vars) s = s.replaceAll("{" + k + "}", vars[k]);
  }
  return s;
};

// Apply translations to any element carrying data-i18n / data-i18n-ph.
window.applyI18n = function (root) {
  const scope = root || document;
  scope.querySelectorAll("[data-i18n]").forEach((el) => {
    el.textContent = window.t(el.getAttribute("data-i18n"));
  });
  scope.querySelectorAll("[data-i18n-ph]").forEach((el) => {
    el.setAttribute("placeholder", window.t(el.getAttribute("data-i18n-ph")));
  });
  document.documentElement.lang = _locale;
};

window.setLocale = function (code) {
  if (!window.GLINT_I18N[code]) return;
  _locale = code;
  try {
    localStorage.setItem("glint.locale", code);
  } catch (e) {}
  window.applyI18n();
};

_locale = detectLocale();
