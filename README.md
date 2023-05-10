# CG Local Arena

This is a local arena for CodinGame challenges.

## How to use

1. Create a folder for the challenge
   ```
    mkdir spring-challenge-2022
    cd spring-challenge-2022
   ```
2. Initialize the arena in the created folder
   ```
    ./cg-local-arena init
   ```
   This will create `config.toml` file.
3. Modify `config.toml` file according to the challenge rules, e.g.:
    ```
    [game]
    title = "Spring Challenge 2022"
    min_players = 2
    max_players = 2
    # Whether the map is symmetric for all the players. If set to false extra mirror matches would be played.
    symmetric = true
    # The command to start the referee process. The referee must respect cg-brutaltester protocol
    referee = "java -jar cg-referee-ghost-in-cell.jar"
    ``` 
4. Run the following and keep it running in the background:
   ```
   ./cg-local-arena run
   ```
5. In another terminal use CLI commands to work with the tool:
   ```
   ./cg-local-arena --help
   ```