package com.codingame.gameengine.runner;

import com.codingame.gameengine.runner.dto.GameResultDto;
import com.google.common.io.Files;
import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Options;
import org.apache.commons.io.FileUtils;
import java.io.File;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.Charset;
import java.nio.file.Paths;
import java.util.List;
import java.util.Properties;

public class CommandLineInterface {

    public static void main(String[] args) {
        try {
            Options options = new Options();

            // Define required options
            options.addOption("h", false, "Print the help")
                    .addOption("p1", true, "Player 1 command line.")
                    .addOption("p2", true, "Player 2 command line.")
                    .addOption("p3", true, "Player 3 command line.")
                    .addOption("p4", true, "Player 4 command line.")
                    .addOption("p5", true, "Player 5 command line.")
                    .addOption("p6", true, "Player 6 command line.")
                    .addOption("p7", true, "Player 7 command line.")
                    .addOption("p8", true, "Player 8 command line.")
                    .addOption("p1n", true, "Player 1 nickname.")
                    .addOption("p2n", true, "Player 2 nickname.")
                    .addOption("p3n", true, "Player 3 nickname.")
                    .addOption("p4n", true, "Player 4 nickname.")
                    .addOption("p5n", true, "Player 5 nickname.")
                    .addOption("p6n", true, "Player 6 nickname.")
                    .addOption("p7n", true, "Player 7 nickname.")
                    .addOption("p8n", true, "Player 8 nickname.")
                    .addOption("league", true, "League level")
                    .addOption("s", false, "Server mode")
                    .addOption("r", true, "File input for replay")
                    .addOption("l", true, "File output for logs")
                    .addOption("seed", true, "Seed");
            // .addOption("port", true, "Port");

            CommandLine cmd = new DefaultParser().parse(options, args);

            int playerCount = 0;
            // CG supports multiplayer games up to 8 players
            for (int i = 1; i <= 8; ++i) {
                if (cmd.hasOption("p" + i)) {
                    playerCount += 1;
                }
            }

            // Rendering the replay
            if (cmd.hasOption("r")) {
                File json = new File(cmd.getOptionValue("r"));
                String jsonResult = FileUtils.readFileToString(json);
                // int port = cmd.hasOption("port")
                // ? Integer.parseInt(cmd.getOptionValue("port"))
                // : 8888;
                new Renderer(8888).render(playerCount, jsonResult);
                return;
            }

            // Launch Game
            MultiplayerGameRunner gameRunner = new MultiplayerGameRunner();

            // Choose league level, depends on the game (19 is max allowed by the engine)
            Integer leagueLevel = Integer.parseInt(cmd.getOptionValue("league", "19"));
            gameRunner.setLeagueLevel(leagueLevel);

            if (cmd.hasOption("seed")) {
                gameRunner.setSeed(Long.valueOf(cmd.getOptionValue("seed")));
            } else {
                gameRunner.setSeed(System.currentTimeMillis());
            }

            GameResultDto result = gameRunner.gameResult;

            // CG supports multiplayer games up to 8 players
            for (int i = 1; i <= 8; ++i) {
                if (cmd.hasOption("p" + i)) {
                    gameRunner.addAgent(cmd.getOptionValue("p" + i), cmd.getOptionValue("p" + i));
                }
            }

            if (cmd.hasOption("s")) {
                gameRunner.start();
            } else {
                Method initialize = GameRunner.class.getDeclaredMethod("initialize", Properties.class);
                initialize.setAccessible(true);
                initialize.invoke(gameRunner, new Properties());

                Method runAgents = GameRunner.class.getDeclaredMethod("runAgents");
                runAgents.setAccessible(true);
                runAgents.invoke(gameRunner);

                if (cmd.hasOption("l")) {
                    Method getJSONResult = GameRunner.class.getDeclaredMethod("getJSONResult");
                    getJSONResult.setAccessible(true);

                    Files.asCharSink(Paths.get(cmd.getOptionValue("l")).toFile(), Charset.defaultCharset())
                            .write((String) getJSONResult.invoke(gameRunner));
                }

                for (int i = 0; i < playerCount; ++i) {
                    System.out.println(result.scores.get(i));
                }

                for (String line : result.uinput) {
                    System.out.println(line);
                }
            }

            // We have to clean players process properly
            Field getPlayers = GameRunner.class.getDeclaredField("players");
            getPlayers.setAccessible(true);
            @SuppressWarnings("unchecked")
            List<Agent> players = (List<Agent>) getPlayers.get(gameRunner);

            if (players != null) {
                for (Agent player : players) {
                    Field getProcess = CommandLinePlayerAgent.class.getDeclaredField("process");
                    getProcess.setAccessible(true);
                    Process process = (Process) getProcess.get(player);

                    process.destroy();
                }
            }
        } catch (Exception e) {
            System.err.println(e);
            e.printStackTrace(System.err);
            System.exit(1);
        }
    }

}