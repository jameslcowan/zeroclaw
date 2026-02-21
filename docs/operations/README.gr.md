# Τεκμηρίωση λειτουργιών και ανάπτυξης (Operations & Deployment)

Για χειριστές που εκτελούν το ZeroClaw σε μόνιμα περιβάλλοντα ή περιβάλλοντα παραγωγής.

## Βασικές λειτουργίες

- Εγχειρίδιο λειτουργίας Day-2 (Runbook): [../operations-runbook.md](../operations-runbook.md)
- Εγχειρίδιο διαδικασίας έκδοσης: [../release-process.md](../release-process.md)
- Πίνακας αντιμετώπισης προβλημάτων: [../troubleshooting.md](../troubleshooting.md)
- Ασφαλής ανάπτυξη δικτύου/πύλης (Gateway): [../network-deployment.md](../network-deployment.md)
- Ρύθμιση Mattermost (ανά κανάλι): [../mattermost-setup.md](../mattermost-setup.md)

## Συνήθης ροή εργασιών

1. Επικύρωση περιβάλλοντος εκτέλεσης (`status`, `doctor`, `channel doctor`)
2. Εφαρμογή μίας αλλαγής παραμέτρων (config) τη φορά
3. Επανεκκίνηση υπηρεσίας/δαίμονα (service/daemon)
4. Επαλήθευση κατάστασης καναλιού και πύλης (gateway health)
5. Γρήγορη επαναφορά (roll back) εάν η συμπεριφορά παρουσιάσει υποτροπή

## Σχετικά

- Αναφορά παραμέτρων (Config reference): [../config-reference.md](../config-reference.md)
- Συλλογή ασφαλείας: [../security/README.md](../security/README.md)
