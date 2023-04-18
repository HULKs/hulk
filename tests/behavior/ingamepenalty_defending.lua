local inspect = require 'inspect'
print("Hello world from lua!")

function spawn_robot(number)
  table.insert(state.robots, create_robot(number))
end

spawn_robot(1)
spawn_robot(2)
spawn_robot(3)
spawn_robot(4)
spawn_robot(5)
spawn_robot(6)
spawn_robot(7)

local game_end_time = -1.0

function on_goal()
  print("Goal scored, resetting ball!")
  print("Ball: " .. inspect(state.ball))
  print("Ball was at x: " .. state.ball.position[1] .. " y: " .. state.ball.position[2])
  state.ball = nil
  game_end_time = state.cycle_count + 200
end

function on_cycle()
  if state.ball == nil and state.cycle_count % 1000 == 0 then
    print(inspect(state))
    state.ball = {
      position = { 0.0, 0.0 },
      velocity = { 0.0, 0.0 },
    }
  end

  if state.cycle_count == 100 then
    state.game_controller_state.game_state = "Ready"
    state.game_controller_state.kicking_team = "Opponent"
    state.filtered_game_state = {
      Ready = {
        kicking_team = "Opponent"
      }
    }
  end

  if state.cycle_count == 1200 then
    state.game_controller_state.game_state = "Set"
    state.filtered_game_state = "Set"
  end

  if state.cycle_count == 1600 then
    state.ball.velocity = { -1.0, 3.0 }
    state.game_controller_state.game_state = "Playing"
    state.filtered_game_state = {
      Playing = {
        ball_is_free = true
      }
    }
  end

  if state.cycle_count == 1900 then
    state.ball.velocity = { -3.0, -0.5 }
  end

  if state.cycle_count == 2000 then
    state.ball.velocity = { -1.0, -4.5 }
  end

  if state.cycle_count == 2300 then
    state.ball.velocity = { -2.0, -0.5 }
  end

  if state.cycle_count == 2650 then
    state.game_controller_state.sub_state = "PenaltyKick"
    state.game_controller_state.kicking_team = "Opponent"
    state.game_controller_state.game_state = "Ready"
    state.filtered_game_state = {
      Ready = {
        kicking_team = "Opponent"
      }
    }
    state.ball = {
      position = { -3.2, 0.0 },
      velocity = { 0.0, 0.0 },
    }
    --Missing behavior starts here
  end

  if state.cycle_count == 4000 then
    state.game_controller_state.game_state = "Set"
    state.filtered_game_state = "Set"
  end

  if state.cycle_count == 4300 then
    state.game_controller_state.game_state = "Playing"
    state.filtered_game_state = {
      Playing = {
        ball_is_free = false
      }
    }
  end

  if state.cycle_count == 5700 then
    state.game_controller_state.sub_state = null
    state.filtered_game_state = {
      Playing = {
        ball_is_free = true
      }
    }
    --"Missing behavior ends here"
  end

  if state.cycle_count == game_end_time then
    state.finished = true
  end

  if state.cycle_count == 15000 then
    state.finished = true
  end
end
