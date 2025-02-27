## Introduction et Contexte

Ce code implémente un éditeur de carte pour le jeu Celeste, en utilisant la bibliothèque graphique **egui** (via **eframe**) pour l'interface et **serde_json** pour manipuler des données JSON. L'éditeur permet de charger, visualiser, modifier et sauvegarder des cartes (en format JSON, puis converties en binaire avec un outil externe nommé *Cairn*).

---

## Importations et Constantes

- **Imports**  
  Le code utilise plusieurs bibliothèques :  
  - `eframe` et `egui` pour l'interface graphique et la gestion des événements (clics, drag, zoom, etc.).  
  - `serde_json` pour la sérialisation/désérialisation des cartes.  
  - Des modules de la bibliothèque standard pour la gestion de fichiers (`std::fs::File` et `std::io::BufReader`) et l'exécution de commandes externes (`std::process::Command`).

- **Constantes**  
  On définit plusieurs constantes pour faciliter la configuration de l'éditeur :
  - `TILE_SIZE` : taille d'une tuile (20 pixels).
  - `GRID_COLOR`, `SOLID_TILE_COLOR`, `BG_COLOR` : couleurs utilisées pour le quadrillage, les tuiles solides et le fond.

---

## La Structure Principale : `CelesteMapEditor`

Cette structure contient l'état de l'éditeur, avec notamment :

- **map_data** : les données de la carte (JSON) chargées depuis un fichier.
- **current_level_index** : l'index de la salle ou du niveau actuellement sélectionné.
- **camera_pos** : la position de la caméra pour le décalage lors du dessin.
- **dragging, drag_start, mouse_pos** : variables pour la gestion du drag (déplacement) de la vue.
- **map_path** : le chemin du fichier de carte ouvert.
- **show_open_dialog** et **error_message** : gestion de l'affichage de la boîte de dialogue d'ouverture de fichier et des messages d'erreur.
- **level_names** : liste des noms des niveaux extraits du JSON.
- **zoom_level** : le niveau de zoom de l'affichage.
- **show_all_rooms** : booléen indiquant si on affiche toutes les salles ou seulement la salle active.

La méthode `default()` est implémentée pour initialiser ces champs avec des valeurs par défaut.

---

## Méthodes de `CelesteMapEditor`

### Chargement et Sauvegarde

- **`load_map(&mut self, path: &str)`**  
  Ouvre le fichier spécifié et tente de parser le contenu en JSON.  
  - En cas de succès, les données sont stockées dans `map_data` et on extrait les noms des niveaux grâce à `extract_level_names()`.
  - En cas d'erreur (ouverture ou parsing), le message d'erreur est stocké dans `error_message`.

- **`save_map(&self)`**  
  Sérialise les données de la carte (JSON) de manière « jolie » et les écrit dans le fichier.  
  - Une fois sauvegardé, le code lance une commande externe (avec *Cairn*) pour convertir le fichier JSON en binaire (`.bin`).

### Gestion des Niveaux et Solides

- **`extract_level_names(&mut self)`**  
  Parcourt le JSON pour extraire le nom de chaque niveau (ou salle) et les stocker dans `level_names`.

- **`get_current_level(&self) -> Option<&Value>`**  
  Retourne le niveau courant en se basant sur `current_level_index`.

- **`get_solids_data(&self) -> Option<String>`**  
  Recherche, dans le niveau courant, le bloc `solids` qui contient la représentation des tuiles sous forme de texte.

- **`update_solids_data(&mut self, new_solids: &str)`**  
  Met à jour la chaîne de caractères représentant les tuiles solides dans le JSON du niveau courant.

### Conversion et Modification de la Carte

- **`screen_to_map(&self, pos: Pos2) -> (i32, i32)`**  
  Convertit des coordonnées écran en coordonnées de tuile en tenant compte du zoom et de la position de la caméra.

- **`place_block(&mut self, pos: Pos2)`**  
  Méthode qui gère le placement d'un bloc.  
  - En mode « toutes les salles », le code détermine quelle salle est cliquée et redirige vers `place_block_in_current_room` après avoir ajusté la position.
  - En mode normal, il place directement le bloc dans la salle courante.

- **`place_block_in_current_room(&mut self, pos: Pos2)`**  
  Convertit la position en coordonnées de tuile, ajuste la chaîne de texte représentant les solides (en remplaçant le caractère à la position par un `'9'` qui représente un bloc solide) et met à jour le JSON.

- **`remove_block(&mut self, pos: Pos2)`**  
  Fonction similaire à `place_block`, mais pour supprimer un bloc (remplacer le caractère par `'0'`).

- **`remove_block_in_current_room(&mut self, pos: Pos2)`**  
  Convertit les coordonnées et modifie la chaîne de tuiles pour remplacer le bloc existant par un vide (`'0'`).

---

## Implémentation de l'Interface Graphique avec `epi::App`

Le trait `epi::App` est implémenté pour intégrer l'éditeur dans la boucle de rendu d’egui.

- **`name(&self) -> &str`**  
  Renvoie le nom de l'application ("Celeste Map Editor").

- **`update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame)`**  
  C’est la fonction principale qui gère :
  
  1. **Le Top Panel**  
     - Un menu « File » pour ouvrir, sauvegarder ou quitter l'application.
     - Un menu « View » pour activer/désactiver l'affichage de toutes les salles, gérer le zoom et réinitialiser la vue.
     - Un combo box permettant de sélectionner le niveau courant si l'affichage est en mode normal.
  
  2. **Le Bottom Panel**  
     - Affichage d’informations en temps réel (position de la souris, coordonnées de la tuile, chemin du fichier ouvert).

  3. **Le Central Panel**  
     - Affichage de l’aire de dessin :
       - Gestion des interactions (clic gauche pour placer un bloc, clic droit pour le supprimer, défilement de la souris pour zoomer, clic et glisser avec le bouton du milieu pour déplacer la caméra).
       - Dessin du fond, du quadrillage (grid) et des éléments de la carte :
         - En mode « toutes les salles », il affiche chaque salle avec son contour et son nom, en coloriant différemment la salle courante.
         - En mode normal, il affiche seulement la salle courante en dessinant chaque bloc solide selon son caractère (avec des couleurs définies dans un `match`).
  
  4. **La Fenêtre de Dialogue d'Ouverture de Fichier**  
     - S'affiche lorsque `show_open_dialog` est activé, permettant de saisir ou parcourir le chemin du fichier de carte à ouvrir.

---

## La Fonction `main`

La fonction `main` crée une instance de l'éditeur (`CelesteMapEditor::new()`) et lance l'application avec `eframe::run_native`, en utilisant les options natives par défaut.
