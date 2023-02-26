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
  print("Ball was at x: " .. state.ball.x)
  state.ball.position[1] = 0;
end

function on_cycle()
  -- if state.time > 100 then
  --   state:return_ball_to_center();
  -- end
  -- print(state.robots)
  -- print("ball: " .. inspect(state.ball))
  if state.ball ~= nil then
    state.ball.x = 0;
  end
end
