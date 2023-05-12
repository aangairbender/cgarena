# CG Local Arena

This is a local arena for CodinGame challenges.

## How to use

1. To create a new arena for a game use `cg-local-arena new`:
   ```shell
   cg-local-arena new spring-challenge-2022
   ```
   Let's check out what CG Local Arena has generated for us:
   ```shell
   cd spring-challenge-2022
   tree .
   .
   ├── config.toml
   └── bots

   1 directory, 1 file
   ```
   Let's check out `config.toml`:
   ```toml
   [game]
   title = "spring-challenge-2022"
   min_players = 2
   max_players = 2
   symmetric = true
   referee = ""
   ```
   This is called arena config, and it contains all of the config that CG Local Arena needs to function.
2. Modify `config.toml` file according to the challenge rules, e.g.:
    ```toml
    [game]
    title = "spring-challenge-2022"
    min_players = 2
    max_players = 2
    # Whether the map is symmetric for all the players. If set to false extra mirror matches would be played.
    symmetric = true
    # The command to start the referee process. The referee must respect cg-brutaltester protocol
    referee = "java -jar cg-referee-ghost-in-cell.jar"
    ``` 
3. Run the following and keep it running in the background:
   ```shell
   cg-local-arena run
   ```
4. In another terminal use CLI commands to work with the tool:
   ```shell
   cg-local-arena --help
   ```

### Adding a bot

To add the bot run the following:
```shell
cg-local-arena bot add "test-bot" -f test.cpp
```