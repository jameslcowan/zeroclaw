# ZeroClaw στο Arduino Uno Q — Οδηγός Βήμα-προς-Βήμα

Εκτελέστε το ZeroClaw στην πλευρά Linux του Arduino Uno Q. Το Telegram λειτουργεί μέσω WiFi, ενώ ο έλεγχος των GPIO γίνεται μέσω του Bridge (απαιτεί μια ελάχιστη εφαρμογή App Lab).

---

## Τι Περιλαμβάνεται (Δεν απαιτούνται αλλαγές κώδικα)

Το ZeroClaw περιλαμβάνει όλα όσα χρειάζονται για το Arduino Uno Q. Κάντε clone το repo και ακολουθήστε αυτόν τον οδηγό — δεν απαιτούνται patches ή προσαρμοσμένος κώδικας.

| Στοιχείο | Τοποθεσία | Σκοπός |
|-----------|----------|---------|
| Bridge app | firmware/zeroclaw-uno-q-bridge/ | MCU sketch + Python socket server (port 9999) για GPIO |
| Bridge tools | src/peripherals/uno_q_bridge.rs | Εργαλεία gpio_read / gpio_write που μιλούν στο Bridge μέσω TCP |
| Setup command | src/peripherals/uno_q_setup.rs | Η εντολή zeroclaw peripheral setup-uno-q αναπτύσσει το Bridge μέσω scp + arduino-app-cli |
| Config schema | board = "arduino-uno-q", transport = "bridge" | Υποστηρίζεται στο config.toml |



Πραγματοποιήστε build με --features hardware για να συμπεριλάβετε την υποστήριξη Uno Q.

---

## Προαπαιτούμενα

- Arduino Uno Q με ρυθμισμένο WiFi
- Arduino App Lab εγκατεστημένο (για την αρχική ρύθμιση και ανάπτυξη)
- API key για LLM (OpenRouter, κ.λπ.)

---

## Φάση 1: Αρχική Ρύθμιση Uno Q (Μία φορά)

### 1.1 Ρύθμιση Uno Q μέσω App Lab

1. Κατεβάστε το Arduino App Lab.
2. Συνδέστε το Uno Q μέσω USB και ενεργοποιήστε το.
3. Ανοίξτε το App Lab και συνδεθείτε στην πλακέτα.
4. Ακολουθήστε τον οδηγό ρύθμισης:
   - Ορίστε όνομα χρήστη και κωδικό πρόσβασης (για SSH)
   - Ρυθμίστε το WiFi (SSID, κωδικός)
   - Εφαρμόστε τυχόν ενημερώσεις υλικολογισμικού
5. Σημειώστε τη διεύθυνση IP που εμφανίζεται (π.χ. arduino@192.168.1.42).

### 1.2 Επαλήθευση Πρόσβασης SSH

ssh arduino@<UNO_Q_IP>
# Εισάγετε τον κωδικό πρόσβασης που ορίσατε

---

## Φάση 2: Εγκατάσταση του ZeroClaw στο Uno Q

### Επιλογή Α: Build στη Συσκευή (Απλούστερο, ~20–40 λεπτά)

# SSH στο Uno Q
ssh arduino@<UNO_Q_IP>

# Εγκατάσταση Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Εγκατάσταση εξαρτήσεων build (Debian)
sudo apt-get update
sudo apt-get install -y pkg-config libssl-dev

# Clone το zeroclaw
git clone https://github.com/theonlyhennygod/zeroclaw.git
cd zeroclaw

# Build (διαρκεί ~15–30 λεπτά στο Uno Q)
cargo build --release --features hardware

# Εγκατάσταση
sudo cp target/release/zeroclaw /usr/local/bin/

---

## Φάση 3: Ρύθμιση του ZeroClaw

### 3.1 Γρήγορη Ρύθμιση (Onboarding)

ssh arduino@<UNO_Q_IP>
zeroclaw onboard --api-key YOUR_OPENROUTER_KEY --provider openrouter

### 3.2 Ελάχιστο αρχείο config.toml

api_key = "YOUR_OPENROUTER_API_KEY"
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4-6"

[peripherals]
enabled = false # Το GPIO μέσω Bridge απαιτεί τη Φάση 4

[channels_config.telegram]
bot_token = "YOUR_TELEGRAM_BOT_TOKEN"
allowed_users = ["*"]

[gateway]
host = "127.0.0.1"
port = 42617

[agent]
compact_context = true

---

## Φάση 4: Εκτέλεση του ZeroClaw Daemon

ssh arduino@<UNO_Q_IP>
zeroclaw daemon --host 127.0.0.1 --port 42617

**Σε αυτό το σημείο:** Το Telegram chat λειτουργεί. Στείλτε μηνύματα στο bot σας — το ZeroClaw αποκρίνεται. Δεν υπάρχει ακόμα έλεγχος GPIO.

---

## Φάση 5: GPIO μέσω Bridge (Το ZeroClaw το χειρίζεται αυτόματα)

### 5.1 Ανάπτυξη του Bridge App

Από τον υπολογιστή σας ή απευθείας από το Uno Q:
zeroclaw peripheral setup-uno-q --host <UNO_Q_IP>

Αυτό αντιγράφει την εφαρμογή Bridge στο ~/ArduinoApps/zeroclaw-uno-q-bridge και την εκκινεί.



### 5.2 Προσθήκη στο config.toml

[peripherals]
enabled = true

[[peripherals.boards]]
board = "arduino-uno-q"
transport = "bridge"

### 5.3 Εκτέλεση

zeroclaw daemon --host 127.0.0.1 --port 42617

Τώρα, όταν στέλνετε μήνυμα στο Telegram "Άναψε το LED", το ZeroClaw χρησιμοποιεί το εργαλείο gpio_write μέσω του Bridge.

---

## Σύνοψη Εντολών

| Βήμα | Εντολή |
|------|---------|
| 1 | Ρύθμιση WiFi/SSH στο App Lab |
| 2 | ssh arduino@<IP> |
| 3 | Εγκατάσταση Rust & Build Deps |
| 4 | git clone & cargo build |
| 5 | zeroclaw onboard |
| 6 | Επεξεργασία config.toml (Telegram token) |
| 7 | zeroclaw peripheral setup-uno-q |
| 8 | zeroclaw daemon |

---

## Αντιμετώπιση Προβλημάτων

- **"command not found: zeroclaw"** — Χρησιμοποιήστε την πλήρη διαδρομή: `/usr/local/bin/zeroclaw` ή βεβαιωθείτε ότι το `~/.cargo/bin` περιλαμβάνεται στο PATH.
- **Το Telegram δεν αποκρίνεται** — Ελέγξτε τα bot_token, allowed_users και βεβαιωθείτε ότι το Uno Q έχει πρόσβαση στο διαδίκτυο (WiFi).
- **Έλλειψη μνήμης (Out of memory)** — Διατηρήστε τα χαρακτηριστικά (features) στο ελάχιστο (`--features hardware` για το Uno Q). Εξετάστε τη ρύθμιση `compact_context = true`.
- **Οι εντολές GPIO αγνοούνται** — Βεβαιωθείτε ότι η εφαρμογή Bridge εκτελείται (η εντολή `zeroclaw peripheral setup-uno-q` την αναπτύσσει και την εκκινεί). Οι ρυθμίσεις πρέπει να έχουν `board = "arduino-uno-q"` και `transport = "bridge"`.
- **Πάροχος LLM (GLM/Zhipu)** — Χρησιμοποιήστε `default_provider = "glm"` ή `"zhipu"` με το `GLM_API_KEY` στις μεταβλητές περιβάλλοντος ή στις ρυθμίσεις. Το ZeroClaw χρησιμοποιεί το σωστό v4 endpoint.