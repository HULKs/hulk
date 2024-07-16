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

local game_end_time = 20000.0

function on_goal()
    print("Goal scored, resetting ball!")
    print("Ball: " .. inspect(state.ball))
    print("Ball was at x: " .. state.ball.position[1] .. " y: " .. state.ball.position[2])
    state.ball = nil
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
    end

    if state.cycle_count == 1600 then
        state.game_controller_state.game_state = "Set"
    end

    if state.cycle_count == 1700 then
        state.game_controller_state.game_state = "Playing"
    end

    if state.cycle_count == 2000 then
        state.game_controller_state.sub_state = "CornerKick"
        state.game_controller_state.kicking_team = "Opponent"
    end

    if state.cycle_count >= 2500 then
        state.ball = {
            position = { -4.5, 3.0},
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 5000 then
        state.game_controller_state.sub_state = "CornerKick"
        state.game_controller_state.kicking_team = "Opponent"
    end

    if state.cycle_count >= 5500 then
        state.ball = {
            position = { -4.5, -3.0},
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 8000 then
        state.game_controller_state.sub_state = "CornerKick"
        state.game_controller_state.kicking_team = "Hulks"
    end

    if state.cycle_count >= 8500 then
        state.ball = {
            position = { 4.5, -3.0},
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 11000 then
        state.game_controller_state.sub_state = "CornerKick"
        state.game_controller_state.kicking_team = "Hulks"
    end

    if state.cycle_count >= 11500 then
        state.ball = {
            position = { 4.5, 3.0},
            velocity = { 0.0, 0.0 },
        }
    end


    if state.cycle_count == 15000 then
        state.game_controller_state.sub_state = "PenaltyKick"
        state.game_controller_state.game_state = "Ready"
        state.game_controller_state.kicking_team = "Opponent"
    end

    if state.cycle_count >= 15000 then
        state.ball = {
            position = { -3.2, 0.0},
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 16500 then
        state.game_controller_state.game_state = "Playing"
    end

    if state.cycle_count == 18000 then
        state.game_controller_state.sub_state = "PenaltyKick"
        state.game_controller_state.game_state = "Playing"
        state.game_controller_state.kicking_team = "Hulks"
    end

    if state.cycle_count >= 18000 then
        state.ball = {
            position = { 3.2, 0.0},
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == game_end_time then
        state.finished = true
    end
end
