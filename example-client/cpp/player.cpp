// To install asio:
// sudo apt install libasio-dev

#include <asio.hpp>
#include <chrono>
#include <cmath>
#include <iostream>
#include <sstream>
#include <string>
#include <thread>
#include <tuple>
#include <vector>

using namespace asio;
using namespace asio::ip;

constexpr double MAX_ACC = 20.0;
constexpr double MAX_SPEED = 100.0;

std::vector<std::string> split(const std::string &s, char delimiter) {
  std::vector<std::string> tokens;
  std::string token;
  std::istringstream tokenStream(s);
  while (std::getline(tokenStream, token, delimiter)) {
    tokens.push_back(token);
  }
  return tokens;
}

std::tuple<
    std::vector<std::tuple<std::string, int, int, int, int, int, int, int>>,
    std::vector<std::tuple<int, int, int>>>
parse_state(const std::vector<std::string> &state_lines) {
  int num_players = std::stoi(state_lines[1]);
  std::vector<std::tuple<std::string, int, int, int, int, int, int, int>>
      players;
  for (int i = 2; i < 2 + num_players; i++) {
    auto tokens = split(state_lines[i], ' ');
    players.emplace_back(tokens[0], std::stoi(tokens[1]), std::stoi(tokens[2]),
                         std::stoi(tokens[3]), std::stoi(tokens[4]),
                         std::stoi(tokens[5]), std::stoi(tokens[6]),
                         std::stoi(tokens[7]));
  }
  int num_items = std::stoi(state_lines[2 + num_players]);
  std::vector<std::tuple<int, int, int>> items;
  for (int i = 3 + num_players; i < 3 + num_players + num_items; i++) {
    auto tokens = split(state_lines[i], ' ');
    items.emplace_back(std::stoi(tokens[0]), std::stoi(tokens[1]),
                       std::stoi(tokens[2]));
  }
  return {players, items};
}

std::tuple<int, int> find_closest_item(
    const std::vector<std::tuple<int, int, int>> &items, int x, int y) {
  int closest_distance = INT_MAX;
  int closest_item_x = -1;
  int closest_item_y = -1;
  for (const auto &[item_x, item_y, _] : items) {
    int dx = item_x - x;
    int dy = item_y - y;
    int distance = dx * dx + dy * dy;
    if (distance < closest_distance) {
      closest_distance = distance;
      closest_item_x = item_x;
      closest_item_y = item_y;
    }
  }
  return {closest_item_x, closest_item_y};
}

int main() {
  const std::string SERVER = "188.166.195.142";
  const int PORT = 7877;
  const std::string LOGIN = "cpp-player";
  const std::string PASSWORD = "cpp-password";

  io_context io_context;

  while (true) {
    try {
      tcp::resolver resolver(io_context);
      tcp::resolver::results_type endpoints =
          resolver.resolve(SERVER, std::to_string(PORT));

      tcp::socket socket(io_context);
      asio::connect(socket, endpoints);

      asio::streambuf response;
      asio::read_until(socket, response, "HELLO");

      asio::write(socket,
                  asio::buffer("PLAY\n" + LOGIN + " " + PASSWORD + "\n"));

      while (true) {
        std::vector<std::string> state_lines;
        while (true) {
          std::string line;
          asio::read_until(socket, response, "\n");
          std::istream response_stream(&response);
          std::getline(response_stream, line);
          if (line.empty() || line == "HELLO") {
            continue;
          }
          state_lines.push_back(line);
          if (line == "END_STATE") {
            break;
          }
        }

        auto [players, items] = parse_state(state_lines);
        auto &[_v0, _v1, x, y, _v2, _v3, _v4, _v5] = players[0];
        auto [target_x, target_y] = find_closest_item(items, x, y);

        std::cerr << "target: " << target_x << " " << target_y << std::endl;
        asio::write(socket,
                    asio::buffer("GO " + std::to_string(target_x) + " " +
                                 std::to_string(target_y) + "\n"));
      }
    } catch (const std::exception &e) {
      std::cerr << "Error: " << e.what() << std::endl;
      std::this_thread::sleep_for(std::chrono::seconds(1));
    }
  }

  return 0;
}
