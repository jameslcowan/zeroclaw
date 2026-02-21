# Αγνωστικιστική Ασφάλεια (Agnostic Security): Μηδενικός αντίκτυπος στη φορητότητα

> ⚠️ **Κατάσταση: Πρόταση / Οδικός Χάρτης (Roadmap)**
>
> Αυτό το έγγραφο περιγράφει προτεινόμενες προσεγγίσεις και ενδέχεται να περιλαμβάνει υποθετικές εντολές ή ρυθμίσεις.
> Για την τρέχουσα συμπεριφορά, δείτε τα: config-reference.md, operations-runbook.md, και troubleshooting.md.

## Βασικό Ερώτημα: Θα προκαλέσουν οι λειτουργίες ασφαλείας προβλήματα σε...
1. ❓ Ταχύτητα cross-compilation builds;
2. ❓ Αρθρωτή αρχιτεκτονική (δυνατότητα αντικατάστασης οποιουδήποτε στοιχείου);
3. ❓ Αγνωστικισμό υλικού (ARM, x86, RISC-V);
4. ❓ Υποστήριξη μικρού υλικού (<5MB RAM, πλακέτες των $10);

**Απάντηση: ΟΧΙ σε όλα** — Η ασφάλεια σχεδιάζεται ως **προαιρετικά feature flags** με **υποθετική μεταγλώττιση (conditional compilation)** ανά πλατφόρμα.

---

## 1. Ταχύτητα Build: Ασφάλεια ελεγχόμενη από Features

### Cargo.toml: Λειτουργίες ασφαλείας πίσω από Features

[features]
default = ["basic-security"]

# Βασική ασφάλεια (πάντα ενεργή, μηδενική επιβάρυνση)
basic-security = []

# Sandboxing ανά πλατφόρμα (opt-in ανά πλατφόρμα)
sandbox-landlock = []  # Μόνο Linux
sandbox-firejail = []  # Μόνο Linux
sandbox-bubblewrap = []# macOS/Linux
sandbox-docker = []    # Όλες οι πλατφόρμες (βαρύ)

# Πλήρης σουίτα ασφαλείας (για production builds)
security-full = [
    "basic-security",
    "sandbox-landlock",
    "resource-monitoring",
    "audit-logging",
]

# Παρακολούθηση πόρων και καταγραφή ελέγχου (audit)
resource-monitoring = []
audit-logging = []

# Development builds (ταχύτερα, χωρίς έξτρα εξαρτήσεις)
dev = []

### Εντολές Build (Επιλέξτε το προφίλ σας)

# Εξαιρετικά γρήγορο dev build (χωρίς πρόσθετα ασφαλείας)
cargo build --profile dev

# Release build με βασική ασφάλεια (προεπιλογή)
cargo build --release
# → Περιλαμβάνει: allowlist, αποκλεισμό διαδρομών, προστασία injection
# → Εξαιρεί: Landlock, Firejail, audit logging

# Production build με πλήρη ασφάλεια
cargo build --release --features security-full
# → Περιλαμβάνει: Τα πάντα

### Υποθετική Μεταγλώττιση: Μηδενική επιβάρυνση όταν είναι απενεργοποιημένη

// src/security/mod.rs

#[cfg(feature = "sandbox-landlock")]
mod landlock;
#[cfg(feature = "sandbox-landlock")]
pub use landlock::LandlockSandbox;

#[cfg(feature = "sandbox-firejail")]
mod firejail;
#[cfg(feature = "sandbox-firejail")]
pub use firejail::FirejailSandbox;

// Πάντα περιλαμβάνεται η βασική ασφάλεια
pub mod policy; // allowlist, path blocking, injection protection

**Αποτέλεσμα**: Όταν οι λειτουργίες είναι απενεργοποιημένες, ο κώδικας δεν μεταγλωττίζεται καν — **μηδενικό "φούσκωμα" (bloat) του binary**.

---

## 2. Αρθρωτή Αρχιτεκτονική: Η Ασφάλεια είναι επίσης ένα Trait

### Security Backend Trait (Εναλλάξιμο όπως όλα τα άλλα)

// src/security/traits.rs

#[async_trait]
pub trait Sandbox: Send + Sync {
    /// Περιτύλιγμα εντολής με προστασία sandbox
    fn wrap_command(&self, cmd: &mut std::process::Command) -> std::io::Result<()>;

    /// Έλεγχος διαθεσιμότητας sandbox σε αυτή την πλατφόρμα
    fn is_available(&self) -> bool;

    /// Αναγνώσιμο όνομα
    fn name(&self) -> &str;
}

// No-op sandbox (πάντα διαθέσιμο - δεν κάνει τίποτα)
pub struct NoopSandbox;

impl Sandbox for NoopSandbox {
    fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
        Ok(()) // Περνάει απαράλλαχτο
    }

    fn is_available(&self) -> bool { true }
    fn name(&self) -> &str { "none" }
}



---

## 3. Αγνωστικισμός Υλικού: Ίδιο Binary, Διαφορετικές Πλατφόρμες

### Μήτρα Συμπεριφοράς Cross-Platform

| Πλαφόρμα | Build OK; | Συμπεριφορά Runtime |
|----------|-----------|---------------------|
| Linux ARM (Raspberry Pi) | ✅ Ναι | Landlock → None (ομαλή μετάπτωση) |
| Linux x86_64 | ✅ Ναι | Landlock → Firejail → None |
| macOS ARM (M1/M2) | ✅ Ναι | Bubblewrap → None |
| Windows x86_64 | ✅ Ναι | None (επίπεδο εφαρμογής) |
| RISC-V Linux | ✅ Ναι | Landlock → None |

**Το ίδιο binary τρέχει παντού** — απλώς προσαρμόζει το επίπεδο προστασίας του βάσει του τι είναι διαθέσιμο στο σύστημα.

---

## 4. Μικρό Υλικό: Ανάλυση Αντικτύπου στη Μνήμη

### Αντίκτυπος στο Μέγεθος Binary (Εκτίμηση)

| Λειτουργία | Μέγεθος Κώδικα | Επιβάρυνση RAM | Κατάσταση |
|------------|----------------|----------------|-----------|
| Base ZeroClaw | 3.4MB | <5MB | ✅ Τρέχουσα |
| + Landlock | +50KB | +100KB | ✅ Linux 5.13+ |
| + Παρακολούθηση πόρων | +30KB | +50KB | ✅ Όλες οι πλατφ. |
| Πλήρης ασφάλεια | +140KB | +350KB | ✅ Παραμένει <6MB |



---

## 5. Εναλλάξιμα Στοιχεία (Agnostic Swaps)

Όπως ακριβώς μπορείτε να αλλάξετε τον πάροχο LLM (OpenAI σε Gemini), μπορείτε να αλλάξετε και το security backend μέσω του config:

# Χρήση Landlock (Linux kernel LSM, native)
[security.sandbox]
backend = "landlock"

# Χρήση Docker (πιο βαρύ, μέγιστη απομόνωση)
[security.sandbox]
backend = "docker"

---

## Σύνοψη: Διατήρηση των Βασικών Αξιών

| Αξία | Πριν | Μετά (με ασφάλεια) | Κατάσταση |
|------------|--------|----------------------|--------|
| <5MB RAM | ✅ <5MB | ✅ <6MB (μέγιστο) | ✅ Διατηρείται |
| <10ms startup | ✅ <10ms | ✅ <15ms | ✅ Διατηρείται |
| ARM + x86 + RISC-V | ✅ Όλα | ✅ Όλα | ✅ Διατηρείται |
| Πλακέτες των $10 | ✅ Λειτουργεί | ✅ Λειτουργεί | ✅ Διατηρείται |
| Πλήρως αρθρωτό | ✅ Ναι | ✅ Ναι (και η ασφάλεια) | ✅ Ενισχυμένο |

**Κάθε στόχος, κάθε πλατφόρμα, κάθε περίπτωση χρήσης — παραμένει γρήγορο, μικρό και αγνωστικιστικό.**