function spawn_robot(number)
  table.insert(state.robots, create_robot(number))
end

spawn_robot(1)
spawn_robot(2)
spawn_robot(3)

function on_cycle()
  if state.cycle_count == 100 then
    state.game_controller_state.game_state = "Ready"
    state.filtered_game_state = {
      Ready = {
        kicking_team = "Hulks",
      },
    }
  end

  if state.cycle_count == 1600 then
    state.filtered_game_state.game_state = "Set"
    state.filtered_game_state = "Set"
    state.ball = {
      position = { 0.0, 0.0 },
      velocity = { 0.0, 0.0 },
    }
  end

  if state.cycle_count == 1700 then
    state.filtered_game_state = {
      Playing = {
        ball_is_free = true,
      },
    }
    state.ball.velocity = { -3.0, 0.1 }
  end

  if state.cycle_count == 1750 then
    state.ball.velocity = { -2.0, 1.0 }
  end

  if state.cycle_count == 1900 then
    state.ball = {
      position = { -3.0, -0.1 },
      velocity = { -2.0, -0.2 },
    }
  end

  if state.cycle_count == 2000 then
    state.finished = true
  end
end
