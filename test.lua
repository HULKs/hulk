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
  -- state:return_ball_to_center();
  print("Ball: " .. inspect(state.ball))
  print("Ball was at x: " .. state.ball.position[1] .. " y: " .. state.ball.position[2])
  state.ball = Null
end

function on_cycle()
  if state.ball == nil and state.cycle_count % 1000 == 0 then
    state.ball = {
      position = { 0.0, 0.0 },
      velocity = { 0.0, 0.0 },
    }
  end
end
