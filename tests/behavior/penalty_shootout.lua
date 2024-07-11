local inspect = require 'inspect'
print("Hello world from lua!")

function spawn_robot(number)
    table.insert(state.robots, create_robot(number))
end

spawn_robot(5)

local game_end_time = 15000
local goal_scored = false

function on_goal()
    print("Goal scored, resetting ball!")
    print("Ball: " .. inspect(state.ball))
    print("Ball was at x: " .. state.ball.position[1] .. " y: " .. state.ball.position[2])
    state.ball = nil
    goal_scored = true
    game_end_time = state.cycle_count + 200
end

function on_cycle()
    if state.ball == nil and state.cycle_count % 1000 == 0 then
        print(inspect(state))
        state.ball = {
            position = { 3.2, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 100 then
        state.game_controller_state.game_state = "Initial"
        state.game_controller_state.kicking_team = "Hulks"
        state.game_controller_state.game_phase = { PenaltyShootout = { kicking_team = "Hulks" } }
        set_robot_pose(5, { 2.8, 0 }, 0);
    end

    if state.cycle_count == 200 then
        state.game_controller_state.game_state = "Set"
    end

    if state.cycle_count == 1700 then
        state.game_controller_state.game_state = "Playing"
    end

    if state.cycle_count == game_end_time then
        if not goal_scored then
            error("No goal was scored!")
        end
        state.finished = true
    end
end
