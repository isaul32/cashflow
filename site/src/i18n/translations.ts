export const LANGS = ['en', 'fi', 'sv', 'de', 'fr', 'es'] as const;
export type Lang = (typeof LANGS)[number];
export const DEFAULT_LANG: Lang = 'en';

/** Language names in their own language. */
export const LANG_NAMES: Record<Lang, string> = {
  en: 'English',
  fi: 'Suomi',
  sv: 'Svenska',
  de: 'Deutsch',
  fr: 'Français',
  es: 'Español',
};

export interface Strings {
  metaTitle: string;
  metaDescription: string;
  ogAlt: string;
  heroTitle: string;
  heroLead: string;
  downloadFor: string; // contains {os}
  getDesktopApp: string;
  viewOnGithub: string;
  otherPlatforms: string;
  availableOn: string;
  free: string;
  screenshotAlt: string;
  featuresTitle: string;
  features: { title: string; text: string }[];
  downloadTitle: string;
  downloadLead: string;
  yourSystem: string;
  winReq: string;
  macReq: string;
  linuxReq: string;
  installer: string;
  installerDetail: string;
  portable: string;
  portableDetail: string;
  dmg: string;
  dmgDetail: string;
  appImage: string;
  appImageDetail: string;
  deb: string;
  debDetail: string;
  macHelpTitle: string;
  macHelpIntro: string;
  macHelpSteps: string[];
  macHelpTerminal: string;
  verify: string;
  buildFromSource: string;
  footerTagline: string;
  footerProduct: string;
  footerFeatures: string;
  footerDownload: string;
  footerReleaseNotes: string;
  footerProject: string;
  footerIssue: string;
  footerLicense: string;
  footerLanguage: string;
  builtApp: string;
  builtSite: string;
  licensed: string;
  languageLabel: string;
  themeLabel: string;
  themeSystem: string;
  themeLight: string;
  themeDark: string;
  bannerText: string;
  bannerSwitch: string;
  dismiss: string;
}

export const strings: Record<Lang, Strings> = {
  en: {
    metaTitle: 'CashFlow: see where your money flows',
    metaDescription:
      'CashFlow is a free, open source desktop app for tracking monthly income and expenses. Your money flow is drawn as a Sankey chart. Works offline, available for Windows, macOS and Linux.',
    ogAlt: 'CashFlow, a desktop app that shows your money flow as a Sankey chart',
    heroTitle: 'See where your money flows.',
    heroLead:
      'A small desktop app for tracking monthly income and expenses. Everything is drawn as a Sankey chart: income sources flow into your total, which flows out to categories and savings.',
    downloadFor: 'Download for {os}',
    getDesktopApp: 'Get the desktop app',
    viewOnGithub: 'View on GitHub',
    otherPlatforms: 'Other platforms',
    availableOn: 'Available for Windows, macOS and Linux',
    free: 'Free and open source · MIT license',
    screenshotAlt: 'CashFlow showing a month of demo data as a Sankey chart',
    featuresTitle: 'Features',
    features: [
      { title: 'Monthly and yearly view', text: 'Add, edit and delete entries per month, or see a whole year in one chart with a per-month breakdown.' },
      { title: 'Your own categories', text: 'Colors and optional sub-categories. The chart gains a level for every nesting.' },
      { title: 'Savings or deficit, automatically', text: 'What is left over appears in the chart without any extra bookkeeping.' },
      { title: 'Six languages', text: 'English, Finnish, Swedish, German, French and Spanish. Numbers, currency and month names follow each language.' },
      { title: 'Your data stays yours', text: 'Automatic saving to plain JSON files on your computer. No account, no cloud.' },
      { title: 'Light and dark theme, seven currencies', text: 'Pick a theme and a currency: EUR, USD, GBP, SEK, NOK, DKK or CHF.' },
    ],
    downloadTitle: 'Download',
    downloadLead: 'Get the latest build for your system. All files are on GitHub Releases.',
    yourSystem: 'Your system',
    winReq: 'Windows 10 and 11, x64',
    macReq: 'Apple Silicon (M1 and newer)',
    linuxReq: 'x64 and arm64',
    installer: 'Installer (.exe)',
    installerDetail: 'Per-user install, no admin rights needed',
    portable: 'Portable (.zip)',
    portableDetail: 'Unzip anywhere and run cashflow.exe',
    dmg: 'Disk image (.dmg)',
    dmgDetail: 'Drag to Applications. See the opening instructions below',
    appImage: 'AppImage',
    appImageDetail: 'Make executable and run',
    deb: '.deb package',
    debDetail: 'Debian, Ubuntu and derivatives',
    macHelpTitle: 'Opening CashFlow on macOS',
    macHelpIntro:
      "The app is not signed with an Apple developer certificate yet, so macOS may say it can't be opened because the developer can't be verified. To open it anyway:",
    macHelpSteps: [
      'Open the downloaded .dmg and drag CashFlow to Applications.',
      'Try to open CashFlow once, then dismiss the warning.',
      'Go to System Settings → Privacy & Security, scroll to Security and click Open Anyway next to CashFlow.',
    ],
    macHelpTerminal: 'Prefer the terminal? Run the command below. This is only needed once.',
    verify: 'Verify downloads with SHA256SUMS.txt.',
    buildFromSource: 'Prefer to build it yourself? See the README.',
    footerTagline: 'A small desktop app that shows where your money flows. Free, open source and offline.',
    footerProduct: 'Product',
    footerFeatures: 'Features',
    footerDownload: 'Download',
    footerReleaseNotes: 'Release notes',
    footerProject: 'Project',
    footerIssue: 'Report an issue',
    footerLicense: 'MIT license',
    footerLanguage: 'Language',
    builtApp: 'App built with Rust and',
    builtSite: 'Site built with',
    licensed: 'MIT licensed.',
    languageLabel: 'Language',
    themeLabel: 'Theme',
    themeSystem: 'System',
    themeLight: 'Light',
    themeDark: 'Dark',
    bannerText: 'This page is also available in English.',
    bannerSwitch: 'Switch to English',
    dismiss: 'Dismiss',
  },
  fi: {
    metaTitle: 'CashFlow: näe minne rahasi virtaavat',
    metaDescription:
      'CashFlow on ilmainen avoimen lähdekoodin työpöytäsovellus kuukausittaisten tulojen ja menojen seurantaan. Rahavirta piirtyy Sankey-kaaviona. Toimii offline-tilassa Windowsilla, macOS:llä ja Linuxilla.',
    ogAlt: 'CashFlow, työpöytäsovellus joka näyttää rahavirtasi Sankey-kaaviona',
    heroTitle: 'Näe minne rahasi virtaavat.',
    heroLead:
      'Pieni työpöytäsovellus kuukausittaisten tulojen ja menojen seurantaan. Kaikki piirtyy Sankey-kaaviona: tulolähteet virtaavat kokonaistuloksi, joka jakautuu menoluokkiin ja säästöihin.',
    downloadFor: 'Lataa {os}-versio',
    getDesktopApp: 'Hae työpöytäsovellus',
    viewOnGithub: 'Avaa GitHubissa',
    otherPlatforms: 'Muut alustat',
    availableOn: 'Saatavilla Windowsille, macOS:lle ja Linuxille',
    free: 'Ilmainen ja avoin lähdekoodi · MIT-lisenssi',
    screenshotAlt: 'CashFlow näyttää kuukauden esimerkkitiedot Sankey-kaaviona',
    featuresTitle: 'Ominaisuudet',
    features: [
      { title: 'Kuukausi- ja vuosinäkymä', text: 'Lisää, muokkaa ja poista kirjauksia kuukausittain tai katso koko vuosi yhdessä kaaviossa kuukausittaisen erittelyn kanssa.' },
      { title: 'Omat luokat', text: 'Värit ja valinnaiset alaluokat. Kaavioon tulee uusi taso jokaisesta sisäkkäisyydestä.' },
      { title: 'Säästö tai alijäämä automaattisesti', text: 'Jäljelle jäävä summa näkyy kaaviossa ilman erillistä kirjanpitoa.' },
      { title: 'Kuusi kieltä', text: 'Englanti, suomi, ruotsi, saksa, ranska ja espanja. Luvut, valuutta ja kuukausien nimet mukautuvat kieleen.' },
      { title: 'Tietosi pysyvät sinulla', text: 'Automaattinen tallennus tavallisiin JSON-tiedostoihin omalle koneellesi. Ei tiliä, ei pilvipalvelua.' },
      { title: 'Vaalea ja tumma teema, seitsemän valuuttaa', text: 'Valitse teema ja valuutta: EUR, USD, GBP, SEK, NOK, DKK tai CHF.' },
    ],
    downloadTitle: 'Lataa',
    downloadLead: 'Hae uusin versio omalle järjestelmällesi. Kaikki tiedostot löytyvät GitHubin julkaisuista.',
    yourSystem: 'Sinun järjestelmäsi',
    winReq: 'Windows 10 ja 11, x64',
    macReq: 'Apple Silicon (M1 ja uudemmat)',
    linuxReq: 'x64 ja arm64',
    installer: 'Asennusohjelma (.exe)',
    installerDetail: 'Käyttäjäkohtainen asennus, ei vaadi järjestelmänvalvojan oikeuksia',
    portable: 'Kannettava (.zip)',
    portableDetail: 'Pura mihin tahansa ja suorita cashflow.exe',
    dmg: 'Levykuva (.dmg)',
    dmgDetail: 'Vedä Ohjelmat-kansioon. Katso avausohje alta',
    appImage: 'AppImage',
    appImageDetail: 'Tee suoritettavaksi ja aja',
    deb: '.deb-paketti',
    debDetail: 'Debian, Ubuntu ja johdannaiset',
    macHelpTitle: 'CashFlow:n avaaminen macOS:llä',
    macHelpIntro:
      'Sovellusta ei ole vielä allekirjoitettu Applen kehittäjäsertifikaatilla, joten macOS voi ilmoittaa, ettei sitä voi avata, koska kehittäjää ei voi vahvistaa. Avaa se silti näin:',
    macHelpSteps: [
      'Avaa ladattu .dmg ja vedä CashFlow Ohjelmat-kansioon.',
      'Yritä avata CashFlow kerran ja sulje varoitus.',
      'Mene kohtaan Järjestelmäasetukset → Yksityisyys ja turvallisuus, vieritä kohtaan Turvallisuus ja valitse CashFlow:n kohdalta Avaa joka tapauksessa.',
    ],
    macHelpTerminal: 'Haluatko käyttää päätettä? Suorita alla oleva komento. Sitä tarvitaan vain kerran.',
    verify: 'Tarkista lataukset tiedostolla SHA256SUMS.txt.',
    buildFromSource: 'Haluatko rakentaa sen itse? Katso README.',
    footerTagline: 'Pieni työpöytäsovellus, joka näyttää minne rahasi virtaavat. Ilmainen, avoin ja offline.',
    footerProduct: 'Tuote',
    footerFeatures: 'Ominaisuudet',
    footerDownload: 'Lataa',
    footerReleaseNotes: 'Julkaisutiedot',
    footerProject: 'Projekti',
    footerIssue: 'Ilmoita ongelmasta',
    footerLicense: 'MIT-lisenssi',
    footerLanguage: 'Kieli',
    builtApp: 'Sovellus tehty Rustilla ja',
    builtSite: 'Sivusto tehty työkalulla',
    licensed: 'MIT-lisenssi.',
    languageLabel: 'Kieli',
    themeLabel: 'Teema',
    themeSystem: 'Järjestelmä',
    themeLight: 'Vaalea',
    themeDark: 'Tumma',
    bannerText: 'Tämä sivu on saatavilla myös suomeksi.',
    bannerSwitch: 'Vaihda suomeen',
    dismiss: 'Sulje',
  },
  sv: {
    metaTitle: 'CashFlow: se vart dina pengar tar vägen',
    metaDescription:
      'CashFlow är en gratis skrivbordsapp med öppen källkod för att följa månatliga intäkter och utgifter. Ditt kassaflöde ritas som ett Sankey-diagram. Fungerar offline på Windows, macOS och Linux.',
    ogAlt: 'CashFlow, en skrivbordsapp som visar ditt kassaflöde som ett Sankey-diagram',
    heroTitle: 'Se vart dina pengar tar vägen.',
    heroLead:
      'En liten skrivbordsapp för att följa månatliga intäkter och utgifter. Allt ritas som ett Sankey-diagram: inkomstkällor flödar in i din totala inkomst, som flödar vidare till kategorier och sparande.',
    downloadFor: 'Ladda ned för {os}',
    getDesktopApp: 'Hämta skrivbordsappen',
    viewOnGithub: 'Visa på GitHub',
    otherPlatforms: 'Andra plattformar',
    availableOn: 'Finns för Windows, macOS och Linux',
    free: 'Gratis och öppen källkod · MIT-licens',
    screenshotAlt: 'CashFlow visar en månads exempeldata som ett Sankey-diagram',
    featuresTitle: 'Funktioner',
    features: [
      { title: 'Månads- och årsvy', text: 'Lägg till, redigera och ta bort poster per månad, eller se ett helt år i ett diagram med uppdelning per månad.' },
      { title: 'Egna kategorier', text: 'Färger och valfria underkategorier. Diagrammet får en nivå för varje nivå av nästling.' },
      { title: 'Sparande eller underskott, automatiskt', text: 'Det som blir över syns i diagrammet utan extra bokföring.' },
      { title: 'Sex språk', text: 'Engelska, finska, svenska, tyska, franska och spanska. Tal, valuta och månadsnamn följer språket.' },
      { title: 'Din data stannar hos dig', text: 'Automatisk sparning till vanliga JSON-filer på din dator. Inget konto, inget moln.' },
      { title: 'Ljust och mörkt tema, sju valutor', text: 'Välj tema och valuta: EUR, USD, GBP, SEK, NOK, DKK eller CHF.' },
    ],
    downloadTitle: 'Ladda ned',
    downloadLead: 'Hämta den senaste versionen för ditt system. Alla filer finns på GitHub Releases.',
    yourSystem: 'Ditt system',
    winReq: 'Windows 10 och 11, x64',
    macReq: 'Apple Silicon (M1 och nyare)',
    linuxReq: 'x64 och arm64',
    installer: 'Installationsprogram (.exe)',
    installerDetail: 'Installation per användare, kräver inga administratörsrättigheter',
    portable: 'Portabel (.zip)',
    portableDetail: 'Packa upp var som helst och kör cashflow.exe',
    dmg: 'Diskavbild (.dmg)',
    dmgDetail: 'Dra till Program. Se instruktionerna nedan',
    appImage: 'AppImage',
    appImageDetail: 'Gör körbar och kör',
    deb: '.deb-paket',
    debDetail: 'Debian, Ubuntu och derivat',
    macHelpTitle: 'Öppna CashFlow på macOS',
    macHelpIntro:
      'Appen är ännu inte signerad med ett Apple-utvecklarcertifikat, så macOS kan säga att den inte kan öppnas eftersom utvecklaren inte kan verifieras. Så här öppnar du den ändå:',
    macHelpSteps: [
      'Öppna den nedladdade .dmg-filen och dra CashFlow till Program.',
      'Försök öppna CashFlow en gång och stäng varningen.',
      'Gå till Systeminställningar → Integritet och säkerhet, scrolla till Säkerhet och klicka på Öppna ändå vid CashFlow.',
    ],
    macHelpTerminal: 'Föredrar du terminalen? Kör kommandot nedan. Det behövs bara en gång.',
    verify: 'Verifiera nedladdningar med SHA256SUMS.txt.',
    buildFromSource: 'Vill du bygga själv? Se README.',
    footerTagline: 'En liten skrivbordsapp som visar vart dina pengar tar vägen. Gratis, öppen och offline.',
    footerProduct: 'Produkt',
    footerFeatures: 'Funktioner',
    footerDownload: 'Ladda ned',
    footerReleaseNotes: 'Versionsanteckningar',
    footerProject: 'Projekt',
    footerIssue: 'Rapportera ett problem',
    footerLicense: 'MIT-licens',
    footerLanguage: 'Språk',
    builtApp: 'Appen är byggd med Rust och',
    builtSite: 'Webbplatsen är byggd med',
    licensed: 'MIT-licens.',
    languageLabel: 'Språk',
    themeLabel: 'Tema',
    themeSystem: 'System',
    themeLight: 'Ljust',
    themeDark: 'Mörkt',
    bannerText: 'Den här sidan finns även på svenska.',
    bannerSwitch: 'Byt till svenska',
    dismiss: 'Stäng',
  },
  de: {
    metaTitle: 'CashFlow: sieh, wohin dein Geld fließt',
    metaDescription:
      'CashFlow ist eine kostenlose Open-Source-Desktop-App zur Erfassung monatlicher Einnahmen und Ausgaben. Dein Geldfluss wird als Sankey-Diagramm dargestellt. Funktioniert offline unter Windows, macOS und Linux.',
    ogAlt: 'CashFlow, eine Desktop-App, die deinen Geldfluss als Sankey-Diagramm zeigt',
    heroTitle: 'Sieh, wohin dein Geld fließt.',
    heroLead:
      'Eine kleine Desktop-App zur Erfassung monatlicher Einnahmen und Ausgaben. Alles wird als Sankey-Diagramm dargestellt: Einnahmequellen fließen in dein Gesamteinkommen, das sich auf Kategorien und Ersparnisse verteilt.',
    downloadFor: 'Für {os} herunterladen',
    getDesktopApp: 'Desktop-App holen',
    viewOnGithub: 'Auf GitHub ansehen',
    otherPlatforms: 'Weitere Plattformen',
    availableOn: 'Verfügbar für Windows, macOS und Linux',
    free: 'Kostenlos und Open Source · MIT-Lizenz',
    screenshotAlt: 'CashFlow zeigt die Beispieldaten eines Monats als Sankey-Diagramm',
    featuresTitle: 'Funktionen',
    features: [
      { title: 'Monats- und Jahresansicht', text: 'Einträge pro Monat hinzufügen, bearbeiten und löschen oder ein ganzes Jahr in einem Diagramm mit Aufschlüsselung nach Monaten sehen.' },
      { title: 'Eigene Kategorien', text: 'Farben und optionale Unterkategorien. Das Diagramm bekommt für jede Verschachtelung eine weitere Ebene.' },
      { title: 'Ersparnis oder Defizit automatisch', text: 'Was übrig bleibt, erscheint im Diagramm, ganz ohne zusätzliche Buchführung.' },
      { title: 'Sechs Sprachen', text: 'Englisch, Finnisch, Schwedisch, Deutsch, Französisch und Spanisch. Zahlen, Währung und Monatsnamen folgen der jeweiligen Sprache.' },
      { title: 'Deine Daten gehören dir', text: 'Automatisches Speichern in einfachen JSON-Dateien auf deinem Computer. Kein Konto, keine Cloud.' },
      { title: 'Helles und dunkles Design, sieben Währungen', text: 'Wähle Design und Währung: EUR, USD, GBP, SEK, NOK, DKK oder CHF.' },
    ],
    downloadTitle: 'Herunterladen',
    downloadLead: 'Hol dir die neueste Version für dein System. Alle Dateien liegen bei den GitHub Releases.',
    yourSystem: 'Dein System',
    winReq: 'Windows 10 und 11, x64',
    macReq: 'Apple Silicon (M1 und neuer)',
    linuxReq: 'x64 und arm64',
    installer: 'Installationsprogramm (.exe)',
    installerDetail: 'Installation pro Benutzer, keine Administratorrechte nötig',
    portable: 'Portabel (.zip)',
    portableDetail: 'Irgendwo entpacken und cashflow.exe starten',
    dmg: 'Disk-Image (.dmg)',
    dmgDetail: 'In „Programme“ ziehen. Siehe die Anleitung unten',
    appImage: 'AppImage',
    appImageDetail: 'Ausführbar machen und starten',
    deb: '.deb-Paket',
    debDetail: 'Debian, Ubuntu und Derivate',
    macHelpTitle: 'CashFlow unter macOS öffnen',
    macHelpIntro:
      'Die App ist noch nicht mit einem Apple-Entwicklerzertifikat signiert, deshalb meldet macOS möglicherweise, dass sie nicht geöffnet werden kann, weil der Entwickler nicht überprüft werden kann. So öffnest du sie trotzdem:',
    macHelpSteps: [
      'Öffne die heruntergeladene .dmg-Datei und ziehe CashFlow in „Programme“.',
      'Versuche, CashFlow einmal zu öffnen, und schließe die Warnung.',
      'Gehe zu Systemeinstellungen → Datenschutz & Sicherheit, scrolle zu Sicherheit und klicke neben CashFlow auf Trotzdem öffnen.',
    ],
    macHelpTerminal: 'Lieber im Terminal? Führe den folgenden Befehl aus. Das ist nur einmal nötig.',
    verify: 'Prüfe Downloads mit SHA256SUMS.txt.',
    buildFromSource: 'Lieber selbst bauen? Siehe die README.',
    footerTagline: 'Eine kleine Desktop-App, die zeigt, wohin dein Geld fließt. Kostenlos, offen und offline.',
    footerProduct: 'Produkt',
    footerFeatures: 'Funktionen',
    footerDownload: 'Herunterladen',
    footerReleaseNotes: 'Release-Notes',
    footerProject: 'Projekt',
    footerIssue: 'Problem melden',
    footerLicense: 'MIT-Lizenz',
    footerLanguage: 'Sprache',
    builtApp: 'App gebaut mit Rust und',
    builtSite: 'Website gebaut mit',
    licensed: 'MIT-Lizenz.',
    languageLabel: 'Sprache',
    themeLabel: 'Darstellung',
    themeSystem: 'System',
    themeLight: 'Hell',
    themeDark: 'Dunkel',
    bannerText: 'Diese Seite gibt es auch auf Deutsch.',
    bannerSwitch: 'Auf Deutsch wechseln',
    dismiss: 'Schließen',
  },
  fr: {
    metaTitle: 'CashFlow : voyez où va votre argent',
    metaDescription:
      "CashFlow est une application de bureau gratuite et open source pour suivre vos revenus et dépenses mensuels. Votre flux d'argent est dessiné sous forme de diagramme de Sankey. Fonctionne hors ligne sur Windows, macOS et Linux.",
    ogAlt: "CashFlow, une application de bureau qui affiche votre flux d'argent sous forme de diagramme de Sankey",
    heroTitle: 'Voyez où va votre argent.',
    heroLead:
      'Une petite application de bureau pour suivre vos revenus et dépenses mensuels. Tout est dessiné sous forme de diagramme de Sankey : les sources de revenus alimentent votre revenu total, qui se répartit ensuite entre catégories et épargne.',
    downloadFor: 'Télécharger pour {os}',
    getDesktopApp: "Obtenir l'application de bureau",
    viewOnGithub: 'Voir sur GitHub',
    otherPlatforms: 'Autres plateformes',
    availableOn: 'Disponible pour Windows, macOS et Linux',
    free: 'Gratuit et open source · Licence MIT',
    screenshotAlt: "CashFlow affichant un mois de données d'exemple sous forme de diagramme de Sankey",
    featuresTitle: 'Fonctionnalités',
    features: [
      { title: 'Vue mensuelle et annuelle', text: 'Ajoutez, modifiez et supprimez des entrées par mois, ou voyez une année entière dans un seul diagramme avec le détail mois par mois.' },
      { title: 'Vos propres catégories', text: 'Des couleurs et des sous-catégories facultatives. Le diagramme gagne un niveau pour chaque imbrication.' },
      { title: 'Épargne ou déficit, automatiquement', text: 'Ce qui reste apparaît dans le diagramme sans comptabilité supplémentaire.' },
      { title: 'Six langues', text: 'Anglais, finnois, suédois, allemand, français et espagnol. Les nombres, la devise et les noms de mois suivent chaque langue.' },
      { title: 'Vos données restent à vous', text: 'Enregistrement automatique dans de simples fichiers JSON sur votre ordinateur. Pas de compte, pas de cloud.' },
      { title: 'Thème clair et sombre, sept devises', text: 'Choisissez un thème et une devise : EUR, USD, GBP, SEK, NOK, DKK ou CHF.' },
    ],
    downloadTitle: 'Télécharger',
    downloadLead: 'Obtenez la dernière version pour votre système. Tous les fichiers sont sur GitHub Releases.',
    yourSystem: 'Votre système',
    winReq: 'Windows 10 et 11, x64',
    macReq: 'Apple Silicon (M1 et plus récent)',
    linuxReq: 'x64 et arm64',
    installer: "Programme d'installation (.exe)",
    installerDetail: "Installation par utilisateur, aucun droit d'administrateur requis",
    portable: 'Portable (.zip)',
    portableDetail: 'Décompressez où vous voulez et lancez cashflow.exe',
    dmg: 'Image disque (.dmg)',
    dmgDetail: 'Glissez dans Applications. Voir les instructions ci-dessous',
    appImage: 'AppImage',
    appImageDetail: 'Rendez-le exécutable et lancez-le',
    deb: 'Paquet .deb',
    debDetail: 'Debian, Ubuntu et dérivés',
    macHelpTitle: 'Ouvrir CashFlow sur macOS',
    macHelpIntro:
      "L'application n'est pas encore signée avec un certificat de développeur Apple, donc macOS peut indiquer qu'elle ne peut pas être ouverte car le développeur ne peut pas être vérifié. Pour l'ouvrir malgré tout :",
    macHelpSteps: [
      'Ouvrez le fichier .dmg téléchargé et glissez CashFlow dans Applications.',
      "Essayez d'ouvrir CashFlow une fois, puis fermez l'avertissement.",
      "Allez dans Réglages Système → Confidentialité et sécurité, faites défiler jusqu'à Sécurité et cliquez sur Ouvrir quand même à côté de CashFlow.",
    ],
    macHelpTerminal: "Vous préférez le terminal ? Exécutez la commande ci-dessous. Cela n'est nécessaire qu'une seule fois.",
    verify: 'Vérifiez les téléchargements avec SHA256SUMS.txt.',
    buildFromSource: 'Vous préférez le compiler vous-même ? Voir le README.',
    footerTagline: "Une petite application de bureau qui montre où va votre argent. Gratuite, ouverte et hors ligne.",
    footerProduct: 'Produit',
    footerFeatures: 'Fonctionnalités',
    footerDownload: 'Télécharger',
    footerReleaseNotes: 'Notes de version',
    footerProject: 'Projet',
    footerIssue: 'Signaler un problème',
    footerLicense: 'Licence MIT',
    footerLanguage: 'Langue',
    builtApp: 'Application développée avec Rust et',
    builtSite: 'Site développé avec',
    licensed: 'Licence MIT.',
    languageLabel: 'Langue',
    themeLabel: 'Thème',
    themeSystem: 'Système',
    themeLight: 'Clair',
    themeDark: 'Sombre',
    bannerText: 'Cette page est aussi disponible en français.',
    bannerSwitch: 'Passer en français',
    dismiss: 'Fermer',
  },
  es: {
    metaTitle: 'CashFlow: mira adónde va tu dinero',
    metaDescription:
      'CashFlow es una aplicación de escritorio gratuita y de código abierto para seguir tus ingresos y gastos mensuales. Tu flujo de dinero se dibuja como un diagrama de Sankey. Funciona sin conexión en Windows, macOS y Linux.',
    ogAlt: 'CashFlow, una aplicación de escritorio que muestra tu flujo de dinero como un diagrama de Sankey',
    heroTitle: 'Mira adónde va tu dinero.',
    heroLead:
      'Una pequeña aplicación de escritorio para seguir tus ingresos y gastos mensuales. Todo se dibuja como un diagrama de Sankey: las fuentes de ingresos fluyen hacia tu ingreso total, que se reparte entre categorías y ahorros.',
    downloadFor: 'Descargar para {os}',
    getDesktopApp: 'Obtener la aplicación de escritorio',
    viewOnGithub: 'Ver en GitHub',
    otherPlatforms: 'Otras plataformas',
    availableOn: 'Disponible para Windows, macOS y Linux',
    free: 'Gratis y de código abierto · Licencia MIT',
    screenshotAlt: 'CashFlow mostrando los datos de ejemplo de un mes como un diagrama de Sankey',
    featuresTitle: 'Características',
    features: [
      { title: 'Vista mensual y anual', text: 'Añade, edita y elimina entradas por mes, o mira un año entero en un solo diagrama con el desglose mensual.' },
      { title: 'Tus propias categorías', text: 'Colores y subcategorías opcionales. El diagrama gana un nivel por cada anidación.' },
      { title: 'Ahorro o déficit, automáticamente', text: 'Lo que sobra aparece en el diagrama sin contabilidad adicional.' },
      { title: 'Seis idiomas', text: 'Inglés, finés, sueco, alemán, francés y español. Los números, la moneda y los nombres de los meses siguen a cada idioma.' },
      { title: 'Tus datos siguen siendo tuyos', text: 'Guardado automático en archivos JSON sencillos en tu ordenador. Sin cuenta, sin nube.' },
      { title: 'Tema claro y oscuro, siete monedas', text: 'Elige un tema y una moneda: EUR, USD, GBP, SEK, NOK, DKK o CHF.' },
    ],
    downloadTitle: 'Descargar',
    downloadLead: 'Obtén la última versión para tu sistema. Todos los archivos están en GitHub Releases.',
    yourSystem: 'Tu sistema',
    winReq: 'Windows 10 y 11, x64',
    macReq: 'Apple Silicon (M1 y posteriores)',
    linuxReq: 'x64 y arm64',
    installer: 'Instalador (.exe)',
    installerDetail: 'Instalación por usuario, no requiere permisos de administrador',
    portable: 'Portátil (.zip)',
    portableDetail: 'Descomprime donde quieras y ejecuta cashflow.exe',
    dmg: 'Imagen de disco (.dmg)',
    dmgDetail: 'Arrastra a Aplicaciones. Consulta las instrucciones de abajo',
    appImage: 'AppImage',
    appImageDetail: 'Hazlo ejecutable y ejecútalo',
    deb: 'Paquete .deb',
    debDetail: 'Debian, Ubuntu y derivadas',
    macHelpTitle: 'Abrir CashFlow en macOS',
    macHelpIntro:
      'La aplicación aún no está firmada con un certificado de desarrollador de Apple, por lo que macOS puede indicar que no se puede abrir porque no se puede verificar al desarrollador. Para abrirla de todos modos:',
    macHelpSteps: [
      'Abre el archivo .dmg descargado y arrastra CashFlow a Aplicaciones.',
      'Intenta abrir CashFlow una vez y cierra el aviso.',
      'Ve a Ajustes del Sistema → Privacidad y seguridad, desplázate hasta Seguridad y haz clic en Abrir de todos modos junto a CashFlow.',
    ],
    macHelpTerminal: '¿Prefieres la terminal? Ejecuta el comando de abajo. Solo hace falta una vez.',
    verify: 'Verifica las descargas con SHA256SUMS.txt.',
    buildFromSource: '¿Prefieres compilarlo tú mismo? Consulta el README.',
    footerTagline: 'Una pequeña aplicación de escritorio que muestra adónde va tu dinero. Gratuita, abierta y sin conexión.',
    footerProduct: 'Producto',
    footerFeatures: 'Características',
    footerDownload: 'Descargar',
    footerReleaseNotes: 'Notas de la versión',
    footerProject: 'Proyecto',
    footerIssue: 'Informar de un problema',
    footerLicense: 'Licencia MIT',
    footerLanguage: 'Idioma',
    builtApp: 'Aplicación creada con Rust y',
    builtSite: 'Sitio creado con',
    licensed: 'Licencia MIT.',
    languageLabel: 'Idioma',
    themeLabel: 'Tema',
    themeSystem: 'Sistema',
    themeLight: 'Claro',
    themeDark: 'Oscuro',
    bannerText: 'Esta página también está disponible en español.',
    bannerSwitch: 'Cambiar a español',
    dismiss: 'Cerrar',
  },
};

/** Path of the page for a language: English lives at the root. */
export function langPath(lang: Lang): string {
  return lang === DEFAULT_LANG ? '/' : `/${lang}/`;
}
