import socket
import numpy as np
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def parse_state(state_lines):
    state_lines = [line.strip() for line in state_lines if line.strip()]
    num_players = int(state_lines[1])
    players = [tuple(map(str, line.split()))
               for line in state_lines[2:2+num_players]]
    num_items = int(state_lines[2+num_players])
    items = [tuple(map(int, line.split()))
             for line in state_lines[3+num_players:3+num_players+num_items]]
    return players, items


def closest_item(player, items):
    player_pos = np.array(player[2:4], dtype=int)
    distances = [np.linalg.norm(np.array(item[:2]) - player_pos)
                 for item in items]
    return items[np.argmin(distances)][:2]


def connect_to_game(server, port, login, password):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect((server, port))

    buffer = ""
    line, buffer = read_line(s, buffer)
    if line != "HELLO":
        logger.error(f"Expected 'HELLO', received '{line}'")
        return None

    s.sendall(b'PLAY\n' + f'{login} {password}\n'.encode())
    return s


def read_line(s, buffer):
    while '\n' not in buffer:
        data = s.recv(4096).decode()
        if not data:
            return None, buffer
        buffer += data

    line, buffer = buffer.split('\n', 1)
    return line.strip(), buffer


def play_game(s):
    buffer = ""
    while True:
        state = []
        line, buffer = read_line(s, buffer)
        while line != "END_STATE":
            if line is None:
                logger.info("Connection closed by server.")
                return
            state.append(line)
            line, buffer = read_line(s, buffer)

        if not state:
            logger.info("Empty state received.")
            continue

        logger.info(f"Received state:\n{' '.join(state)}")

        players, items = parse_state(state)
        my_player = players[0]
        if items:
            target_x, target_y = closest_item(my_player, items)
            s.sendall(f"GO {target_x} {target_y}\n".encode())
            logger.info(f"Sent target coordinates: ({target_x}, {target_y})")
        else:
            logger.info("No items found in the state.")


if __name__ == "__main__":
    SERVER = "188.166.195.142"
    PORT = 7877
    LOGIN = "your_login"
    PASSWORD = "your_password"

    logger.info("Starting the bot...")
    s = connect_to_game(SERVER, PORT, LOGIN, PASSWORD)
    if s:
        logger.info("Connected to the game server.")
        play_game(s)
    else:
        logger.error("Failed to connect to the game server.")
