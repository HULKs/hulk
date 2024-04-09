local inspect = require 'inspect'

function spawn_robot(number)
    table.insert(state.robots, create_robot(number))
end

spawn_robot(2)

local game_end_time = 10000
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
    if state.cycle_count == 1000 then
        state.ball = {
            position = { 0.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 2300 then
        state.ball = {
            position = { -3.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 1 then
        state.game_controller_state.game_state = "Ready"
        state.filtered_game_state = {
            Ready = {
                kicking_team = "Hulks",
            }
        }
    end

    if state.cycle_count == 1150 then
        state.filtered_game_state.game_state = "Set"
        state.filtered_game_state = "Set"
    end

    if state.cycle_count == 1200 then
        state.filtered_game_state = {
            Playing = {
                ball_is_free = true,
                kick_off = true
            }
        }
    end

    if state.cycle_count == game_end_time then
        -- if not goal_scored then
        --   error("No goal was scored!")
        -- end
        state.finished = true
    end
end
