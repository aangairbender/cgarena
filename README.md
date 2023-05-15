# CG Arena

This is a local arena for CodinGame challenges.

## How to use

1. To create a new arena for a game use `cgarena new`:
   ```shell
   cgarena new spring-challenge-2022
   ```
   Let's check out what CG Arena has generated for us:
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
   This is called arena config, and it contains all of the config that CG Arena needs to function.
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
   cgarena run
   ```
4. In another terminal use CLI commands to work with the tool:
   ```shell
   cgarena help
   ```

### Adding a bot

To add the bot run the following:
```shell
cgarena bot add "test-bot" -f test.cpp
```

### Adding a worker

Start the worker on another pc in local network:
```shell
cgarena-worker -t 4
```

To add the worker run the following:
```shell
cgarena worker add "local-laptop" -h <host> -p <port>
```