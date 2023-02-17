print("Hello world from lua!")

state:spawn_robot(1);
state:spawn_robot(2);
state:spawn_robot(3);
state:spawn_robot(4);
state:spawn_robot(5);

-- on_cycle(enforce_spl_rules);

-- state:on_goal(function()
--   print("Goal scored!!!");
--   state:return_ball_to_center(0);
-- end);

on_goal = function()
  print("Goal scored, resetting ball!")
  state:return_ball_to_center();
end;
