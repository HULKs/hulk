local inspect = require 'inspect'
print("Hello world from lua!")

function spawn_robot(number)
  table.insert(state.robots, new_robot(number))
end

spawn_robot(1)
spawn_robot(2)
spawn_robot(3)
spawn_robot(4)
spawn_robot(5)

function on_goal()
  print("Goal scored, resetting ball!")
  print("Ball: " .. inspect(state.ball))
  print("Ball was at x: " .. state.ball.position[1] .. " y: " .. state.ball.position[2])
  state.ball = Null
end

function on_cycle()
  if state.ball == nil and state.cycle_count % 1000 == 0 then
    print(inspect(state))
    state.ball = {
      position = { 0.0, 0.0 },
      velocity = { 0.0, 0.0 },
    }
  end

  if state.cycle_count == 1000 then
    state.filtered_game_state = {
      Playing = {
        ball_is_free = true
      }
    }
  end

  if state.cycle_count == 3000 then
    state.game_controller_state.set_play = "PushingFreeKick"
    state.game_controller_state.kicking_team = "Opponent"
    state.filtered_game_state.Playing.ball_is_free = false
  end

  if state.cycle_count == 5000 then
    state.filtered_game_state.Playing.ball_is_free = true
    set_robot_pose(1, {3.0, 2.5}, 0.0)
  end

  if state.cycle_count == 7000 then
    set_robot_penalized(4, true);
    set_robot_penalized(5, true);
  end

  if state.cycle_count == 10000 then
    state.finished = true
  end
end
