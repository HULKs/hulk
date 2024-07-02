-- local inspect = require 'inspect'
print("Hello world from lua!")

function spawn_robot(number)
    table.insert(state.robots, create_robot(number))
end

spawn_robot(1)
spawn_robot(7)

local game_end_time = 15000
local goal_scored = false

function on_goal()
    print("Goal scored, resetting ball!")
    -- print("Ball: " .. inspect(state.ball))
    print("Ball was at x: " .. state.ball.position[1] .. " y: " .. state.ball.position[2])
    state.ball = nil
    goal_scored = true
    game_end_time = state.cycle_count + 200
end

function on_cycle()
    if state.ball == nil and state.cycle_count % 1000 == 0 then
        -- print(inspect(state))
        state.ball = {
            position = { 0.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 100 then
        state.game_controller_state.game_state = "Ready"
    end

    if state.cycle_count == 1600 then
        state.game_controller_state.game_state = "Set"
    end

    if state.cycle_count == 1700 then
        state.game_controller_state.game_state = "Playing"
    end

    if state.cycle_count == 1701 then
        penalize(7)
        state.game_controller_state.penalties.seven = {
            Manual = {
                remaining = {
                    nanos = 0,
                    secs = 5
                },
            }
        };
        set_robot_pose(7, { -3.2, 3 }, -1.5707963267948966);
        state.ball = {
            position = { -2.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 2000 then
        state.ball = nil
    end

    if state.cycle_count == 3000 then
        unpenalize(7)
        state.game_controller_state.penalties.seven = nil
    end

    if state.cycle_count == game_end_time then
        if not goal_scored then
            error("No goal was scored!")
        end
        state.finished = true
    end
end
