print("Hello world from lua!")

state:spawn_robot(1)
state:spawn_robot(2)
state:spawn_robot(3)
state:spawn_robot(4)
state:spawn_robot(5)

function on_goal()
  print("Goal scored, resetting ball!")
  -- state:return_ball_to_center();
  print("Ball was at x: " .. state.ball.x)
  state.ball.x = 0;
end

function on_cycle()
  if state.time > 100 then
    state:return_ball_to_center();
  end
end
