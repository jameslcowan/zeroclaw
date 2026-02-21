# Πρώτα Βήματα (Getting Started)

Οδηγίες για την πρώτη σας εγκατάσταση και για μια γρήγορη γνωριμία με το σύστημα.

## Πώς να Ξεκινήσετε

1. **Γενική παρουσίαση και γρήγορη εκκίνηση:** [../../README.md](../../README.md)
2. **Εγκατάσταση με ένα κλικ και διπλή λειτουργία (dual bootstrap mode):** [../one-click-bootstrap.md](../one-click-bootstrap.md)
3. **Εύρεση εντολών ανά εργασία:** [../commands-reference.md](../commands-reference.md)

---

## Πώς vα προχωρήσετε

| Τι θέλετε να κάνετε | Εντολή |
| :--- | :--- |
| Έχω κλειδί API και θέλω την πιο γρήγορη εγκατάσταση | `zeroclaw onboard --api-key sk-... --provider openrouter` |
| Θέλω βοήθεια βήμα-βήμα με ερωτήσεις | `zeroclaw onboard --interactive` |
| Οι ρυθμίσεις υπάρχουν, θέλω μόνο να φτιάξω τα κανάλια | `zeroclaw onboard --channels-only` |
| Οι ρυθμίσεις υπάρχουν, αλλά θέλω να τις αντικαταστήσω όλες | `zeroclaw onboard --force` |
| Χρήση με κωδικούς συνδρομής | Δείτε το [Subscription Auth](../../README.md#subscription-auth-openai-codex--claude-code) |

---

## Ρύθμιση και Έλεγχος

* **Γρήγορη ρύθμιση:** `zeroclaw onboard --api-key "sk-..." --provider openrouter`
* **Διαδραστική (interactive) ρύθμιση με ερωτήσεις:** `zeroclaw onboard --interactive`
* **Προστασία ρυθμίσεων:** Αν ξανατρέξετε τη διαδικασία, το σύστημα θα σας ζητήσει επιβεβαίωση (εκτός αν χρησιμοποιήσετε την εντολή `--force`).
* **Μοντέλα Ollama cloud (`:cloud`):** Χρειάζονται μια διεύθυνση `api_url` και ένα κλειδί API (π.χ. `api_url = "https://ollama.com"`).
* **Έλεγχος καλής λειτουργίας:** Χρησιμοποιήστε τις εντολές `zeroclaw status` και `zeroclaw doctor` για να βεβαιωθείτε ότι όλα δουλεύουν σωστά.

## Επόμενα Βήματα

- Λειτουργίες Runtime: [../operations/README.md](../operations/README.md)
- Κατάλογοι αναφοράς: [../reference/README.md](../reference/README.md)
