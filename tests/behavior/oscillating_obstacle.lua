function spawn_robot(number)
  table.insert(state.robots, create_robot(number))
end

spawn_robot(7)

state.filtered_game_state = {
  Playing = {
    ball_is_free = true,
  },
}

function on_cycle()
  if state.cycle_count == 1 then
    state.ball = {
      position = { 1.0, 0.0 },
      velocity = { 0.0, 0.0 },
    }
    set_robot_pose(7, { -1.0, 0 }, 0)

    create_obstacle(7, { 0.1, 0.0 }, 0.3)
  end

  if state.cycle_count > 100 and state.cycle_count % 10 == 0 then
    clear_obstacles(7)
    create_obstacle(7, { 0.1, math.random() * 0.05 }, 0.3)
  end

  if state.cycle_count == 5000 then
    state.finished = true
  end
end

function on_goal()
  state.finished = true
end
