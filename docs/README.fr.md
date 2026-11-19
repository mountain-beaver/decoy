<p align="center"><a href="../README.md">English</a> · <a href="README.ru.md">Русский</a> · <a href="README.pt-BR.md">Português</a> · <a href="README.es.md">Español</a> · <b>Français</b> · <a href="README.de.md">Deutsch</a></p>

<p align="center">
  <img src="../assets/icon.svg" width="112" alt="Le logo de Decoy : un canard blanc au bec jaune">
</p>

<h1 align="center">Decoy</h1>

<p align="center">
  <b>Affiche ce que tu veux comme statut Discord.</b><br>
  Un jeu auquel tu ne joues pas, un bouton qui mène à ton serveur, ton stream, ta propre image et ton propre texte.<br>
  Gratuit et léger. Pas de compte, pas de mot de passe, pas de code.
</p>

<p align="center">
  <a href="https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-windows-x64-setup.exe"><img src="download-windows.png" width="250" alt="Télécharger pour Windows"></a>
  &nbsp;
  <a href="https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-darwin-universal.dmg"><img src="download-macos.png" width="250" alt="Télécharger pour macOS"></a>
  &nbsp;
  <a href="https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-amd64.AppImage"><img src="download-linux.png" width="250" alt="Télécharger pour Linux"></a>
</p>

<p align="center">
  <sub>Windows 10 et 11, macOS 10.15 ou plus récent, la plupart des systèmes Linux. <a href="#install">Aide à l'installation et autres fichiers</a></sub>
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="window-dark.png">
    <img src="window-light.png" width="880" alt="La fenêtre de Decoy : tu remplis le formulaire à gauche et tu vois ton statut Discord à droite">
  </picture>
</p>

Decoy est une appli gratuite et open source pour la Rich Presence personnalisée de Discord. Elle règle ton statut d'activité Discord (la ligne « Joue à… » sous ton nom) sur n'importe quel jeu, ou sur ton propre nom, texte, images et boutons. Si tu connais CustomRP pour Windows, Decoy fait le même travail et fonctionne aussi sur Mac et Linux.

## Ce que voient tes amis

<p align="center">
  <img src="discord.png" width="720" alt="Trois morceaux d'une vraie fenêtre Discord : la carte Joue à sur un profil, une ligne de salon vocal avec l'icône du jeu à côté du nom et une ligne de la liste des membres">
</p>

Ce sont des morceaux d'une vraie fenêtre Discord, avec le nom et l'avatar masqués. Ton statut s'affiche sur ton profil, dans la liste des membres et sous la forme d'une petite icône à côté de ton nom dans les salons vocaux.

## À quoi ça sert

<p align="center">
  <img src="ideas.png" width="860" alt="Trois exemples de statut : un jeu de course, un serveur appelé Moonlight Lounge avec un bouton Rejoins mon serveur et une radio en direct avec deux boutons">
</p>

- **Avoir l'air occupé.** Choisis parmi environ 25 000 jeux que Discord connaît. Ton statut a exactement l'air de celui de quelqu'un qui joue.
- **Inviter des gens sur ton serveur.** Mets sur ton profil un bouton qui ouvre ton lien d'invitation. [Voici comment faire](#add-a-link-to-your-server).
- **Partager ce que tu aimes.** Un stream, une playlist, une chaîne YouTube, ton site : n'importe quel lien peut devenir un bouton.
- **Le faire à ta façon.** Ton propre nom, ta propre image et ton propre texte à la place d'un jeu. Un guide dans Decoy t'explique comment, étape par étape.

## Pour commencer

1. **Télécharge Decoy** avec un des boutons ci-dessus et ouvre-le. La première fois, Windows ou macOS peuvent te demander si tu lui fais confiance : [sur quoi appuyer](#install).
2. **Choisis un jeu.** Commence à taper son nom et choisis-le dans la liste.
3. **Écris ton texte**, si tu veux : deux lignes, un minuteur, un bouton. L'aperçu à droite montre ce que les autres verront.
4. **Active « Showing in Discord »** (**Showing in Discord** veut dire « Affiché dans Discord ») en haut à droite. C'est tout.

Discord doit être ouvert sur le même ordinateur (l'appli, pas le site). Tu peux fermer la fenêtre de Decoy : il continue de tourner dans la zone de notification, à côté de l'horloge.

<a id="add-a-link-to-your-server"></a>

## Ajouter un lien vers ton serveur

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="buttons-dark.png">
    <img src="buttons-light.png" width="680" alt="La section Buttons dans Decoy : le libellé est Join my server et le lien est une invitation Discord">
  </picture>
</p>

1. Dans Discord, ouvre ton serveur, clique sur son nom en haut et choisis **Inviter des gens**. Clique sur **Edit invite link** (modifier le lien d'invitation), règle-le pour qu'il n'expire jamais et copie le lien.
2. Dans Decoy, descends jusqu'à **Buttons** (boutons) et appuie sur **Add button** (ajouter un bouton).
3. Dans **Label** (libellé), écris ce que le bouton doit dire, par exemple `Rejoins mon serveur`. Dans **Link** (lien), colle ton invitation.
4. Vérifie que **Showing in Discord** est activé.

Bon à savoir :

- Tu ne peux pas cliquer sur ton propre bouton. Discord ne laisse que les autres personnes cliquer dessus, alors demande à un ami de vérifier.
- Tu peux avoir deux boutons. N'importe quel lien qui commence par `https://` fonctionne : une invitation de serveur, Twitch, YouTube, ton site.
- Tu veux le nom et le logo de ton serveur à la place d'un jeu ? Appuie sur **Guide** dans Decoy, puis sur **Own app** (appli perso). C'est gratuit et ça prend environ cinq minutes.

<a id="install"></a>

## Installation

| Ton ordinateur | Téléchargement | La première fois que tu l'ouvres |
|---|---|---|
| Windows 10 et 11 | [Télécharger le .exe](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-windows-x64-setup.exe) | Windows peut afficher « Windows a protégé votre ordinateur ». Appuie sur **Informations complémentaires**, puis sur **Exécuter quand même**. |
| Mac (Intel et Apple Silicon) | [Télécharger le .dmg](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-darwin-universal.dmg) | Glisse Decoy dans Applications. Si macOS dit qu'il ne peut pas vérifier l'appli, ouvre **Réglages Système**, **Confidentialité et sécurité**, descends et appuie sur **Ouvrir quand même**. |
| Linux, tous | [Télécharger l'.AppImage](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-amd64.AppImage) | Autorise l'exécution du fichier (`chmod +x`), puis ouvre-le. |
| Debian, Ubuntu, Mint | [Télécharger le .deb](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-amd64.deb) | `sudo apt install ./Decoy-linux-amd64.deb` |
| Fedora, openSUSE | [Télécharger le .rpm](https://github.com/mountain-beaver/decoy/releases/latest/download/Decoy-linux-x86_64.rpm) | `sudo dnf install ./Decoy-linux-x86_64.rpm` |

**Pourquoi Windows et macOS affichent-ils un avertissement ?** Ils préviennent pour chaque appli dont le créateur n'a pas payé de certificat à Microsoft ou à Apple, et Decoy est un projet gratuit qui n'en a pas payé. Les installateurs sont construits automatiquement à partir du code de ce dépôt, que tout le monde peut lire.

Toutes les versions sont sur la page [Releases](https://github.com/mountain-beaver/decoy/releases).

## Questions

**C'est gratuit ?**<br>
Oui. Pas de pub, pas de version payante.

**Est-ce sans danger pour mon compte ?**<br>
Decoy ne te demande jamais ton mot de passe Discord et ne se connecte pas à ton compte. Il dit simplement à l'appli Discord de ton ordinateur quoi afficher, comme le font les jeux.

**Dois-je le laisser ouvert ?**<br>
Decoy doit tourner, mais tu peux fermer sa fenêtre : il reste dans la zone de notification. Pour qu'il démarre avec ton ordinateur, ouvre **Settings** (réglages, la roue dentée) et active **Start with my computer** (démarrer avec mon ordinateur).

**Ça marche sur téléphone ou dans le navigateur ?**<br>
Decoy tourne sur un ordinateur, à côté de l'appli Discord de bureau. Les gens sur téléphone verront quand même ton statut.

**Rien ne s'affiche dans Discord. Que vérifier ?**
1. L'appli Discord de bureau tourne sur le même ordinateur.
2. Dans Discord, **Paramètres utilisateur**, **Activity Privacy** (confidentialité de l'activité) : le partage de ton activité est activé.
3. Ton statut n'est pas **Invisible**.

Il y a une liste de vérifications plus longue dans Decoy : **Guide**, puis **Not showing** (rien ne s'affiche).

**Puis-je avoir plusieurs statuts ?**<br>
Oui. Crée plusieurs profils et passe de l'un à l'autre dans la fenêtre ou depuis l'icône de la zone de notification.

**Puis-je changer le nom après « Joue à » ?**<br>
Oui, avec ta propre application : **Guide**, puis **Own app**.

## Vie privée

Decoy n'a ni compte, ni outil d'analyse, ni serveur à lui. Tes profils sont enregistrés uniquement sur ton ordinateur.

Il se connecte à internet pour deux choses : à Discord, pour charger la liste des jeux, les noms et les images, et à GitHub, au plus deux fois par jour, pour voir s'il existe une version plus récente. Il te montre seulement un lien et ne télécharge jamais rien tout seul. Tu peux désactiver cette vérification dans **Settings**.

## Il te plaît ?

Mets une étoile au dépôt en haut de cette page. Ça ne coûte rien et ça aide d'autres personnes à trouver Decoy. Tu as trouvé un bug ou tu as une idée ? [Dis-le-moi ici](https://github.com/mountain-beaver/decoy/issues/new/choose), pas besoin de détails techniques.

## Pour les développeurs

Decoy est open source sous [licence MIT](../LICENSE). Comment il fonctionne en interne, comment le compiler et comment lancer les vérifications : [DEVELOPMENT.md](DEVELOPMENT.md).

<sub>Decoy n'est affilié ni à Discord ni à aucun éditeur de jeux. Les noms et les icônes des jeux, y compris ceux des images ci-dessus, appartiennent à leurs propriétaires.</sub>
