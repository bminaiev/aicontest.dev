# Intro

In this contest, you need to write a program (using any programming language), which
connects to the server via TCP and plays a game. In this game, each player is represented
as a circle flying around a rectangular field.

There are some items (also represented
as circles) randomly placed on the field. Whenever a player intersects with an item,
the item is removed from the field and the player's score is increased by one.

You can see the current game in real-time: https://aicontest.dev/

![image](https://user-images.githubusercontent.com/2011126/236305046-41a362c5-cb8c-4bae-9722-865cbf47f3dc.png)


# Navigating through the field

The game consists of **600** moves. Each move roughly takes **0.5s**. Each player has position **(x, y)** and current speed **(vx, vy)**. **You can't instantly change the speed direction!** On each turn you can specify the target position **(target_x, target_y)**. The new position and new speed for the next turn are calculated like this:

- First, acceleration direction is calculated as **(target_x - x, target_y - y)**.
- Second, acceleration **(ax, ay)** is calculated as acceleration direction multiplied by some coefficient to make it less than **MAX_ACC = 20.0**. Acceleration components are rounded to the nearest integers.
- New speed is calculated as **(vx + ax, vy + ay)**.
- If the speed is bigger than **MAX_SPEED = 100.0**, it is clamped by **MAX_SPEED**.
- New coordinates are calculated as **(x + vx, y + vy)**.
- If the player tries to fly outside of the field, it bounces off the side.

See details in the implementation: https://github.com/bminaiev/aicontest.dev/blob/master/common/src/game_state.rs#L112

# Connection to the server

Your program should connect to the **188.166.195.142** at port **7877**.

The server sends the message **HELLO**.

Your program should send **PLAY** on the first line.
On the second line you should send two words **[LOGIN] [PASSWORD]**. You can use any **[LOGIN]** which is not used yet.

```
Please don't use a password, which you use somewhere else. Just generate a new random password. 
Passwords are stored in plaintext on the server. You need to use the same password 
every time you connect to the server.
```

After that server sends the current state of the game using this format:

```
TURN [CUR_TURN] [MAX_TURNS] [WIDTH] [HEIGHT] [GAME_ID]
[NUM_PLAYERS]
[PLAYER_NAME] [SCORE] [X] [Y] [RADIUS] [V_X] [V_Y] [TARGET_X] [TARGET_Y]
... ([NUM_PLAYERS - 1] more lines)
[NUM_ITEMS]
[ITEM_X] [ITEM_Y] [RADIUS]
... ([NUM_ITEMS - 1] more lines)
END_STATE
```

The first player in the state is always your player.

**[GAME_ID]** is just a random string.

You can update the **[TARGET_X] [TARGET_Y]** by sending the command **GO [NEW_TARGET_X] [NEW_TARGET_Y]**.

After that, the server updates the target, calculates the next state, and sends it back in the same format.

## Example of the interaction

If you are using Linux you can play from a command line using `nc` like this:

```
$ nc 188.166.195.142 7877
HELLO
                                                            PLAY
                                                       test test
TURN 398 600 4195 3146 game-2023-05-04_18-42-18
3
test 0 390 1932 20 0 0 390 1932
basic-rust-45 16 2487 952 20 91 -14 2436 834
basic-rust-185 14 1868 599 20 -94 35 1615 499
5
4062 1072 25
3747 2866 66
1395 1460 20
3666 2242 25
1007 1635 21
END_STATE


                                                      GO 100 100
TURN 405 600 4195 3146 game-2023-05-04_18-42-18
3
test 0 390 1932 20 0 0 390 1932
basic-rust-45 17 2994 838 20 55 30 2770 1116
basic-rust-185 15 1206 519 20 -99 -14 1214 633
5
4062 1072 25
3747 2866 66
1395 1460 20
3666 2242 25
3462 2904 73
END_STATE
```

## Clients example

- Rust: https://github.com/bminaiev/aicontest.dev/tree/master/example-client

# Notes

- Please do not try to destabilize the system!
- Please do not hardcode the size of the field. It could change during the game based on the number of players.
- We ask for **(target_x, target_y)** instead of **(ax, ay)** to make it possible to play even if the latency to the server is bigger than one turn time. If you don't send a new target, the target from the previous turn is used, which could be a reasonable choice.
- Sometimes we will restart the server, consider adding a reconnection logic to your program. Please sleep for a couple of seconds before reconnection.
