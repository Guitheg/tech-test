# Gobin Tech Interview

## Comments (en Français)
Alors a propos des requirements:
 1. je suis pas satisfait de ce que j'ai fais. Il faudrait que j'améliorer la partie aync
 2. j'ai pas la partie signature, et la partie process, je suis pas vraiment satisfait de ce que j'ai fais, faudrait faire plus de vérification, être un peu plus abstrait parfois (comme pour laisser interagir avec une base de données)
 3. Api Restful ne fonctionne pas, je renvois juste un string sans construire la réponse http adéquate. Et en plus, j'ai pas fait health.
 4. la documentation est pauvre car je manquais de temps et je prévilégié le reste

 Optionel: j'aurais voulu le faire mais manque de temps

Le point positif c'est que je n'ai pas été largué. Je comprenais ce qu'on me demandait, et je savais plus ou moins comment faire.
Le point négatif c'est que j'ai du apprendre beaucoup de truc, je connaissais pas du tout les libs, j'ai du partir de zéro sur tout à chaque fois.
J'ai perdu du temps car j'ai commencé à partir sur ethereum test net pour le blockchain listener puis je me suis rendu compte que l'event OpPoked était pas très actif, donc j'ai préféré prendre SubmittedSpotEntry sur starknet. ça m'a fait perdre beaucoup de temps mais au moins j'ai bien appris et vu comment faire.

Je suis plutôt déçu dans l'ensemble, mais je suis content car c'était plutôt productif en terme d'apprentissage, on apprends toujours mieux quand il y a des tâches données.


## Launch

Make sure you have set your `INFURA_API_KEY` env variable

Launch the server with a 'REST' Api
```
sh launch.sh
```

On another terminal, to get the data from the api:

```
python script/get_data.py
```
