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

local game_end_time = 15000.0

function on_goal()
    print("Goal scored, resetting ball!")
    print("Ball: " .. inspect(state.ball))
    print("Ball was at x: " .. state.ball.position[1] .. " y: " .. state.ball.position[2])
    state.ball = nil
    game_end_time = state.cycle_count + 200
end

state.ball = {
    position = { 0.0, 0.0 },
    velocity = { 0.0, 0.0 },
}

function on_cycle()
    if state.cycle_count % 1000 == 0 then
        print(inspect(state))
    end

    if state.cycle_count == 100 then
        state.filtered_game_controller_state.game_state = {
            Ready = {
                kicking_team = "Hulks"
            }
        }
    end

    if state.cycle_count == 1600 then
        state.filtered_game_controller_state.game_state = "Set"
    end

    if state.cycle_count == 1700 then
        state.filtered_game_controller_state.game_state = {
            Playing = {
                ball_is_free = true,
                kick_off = false,
            }
        }
    end

    if state.cycle_count == 1900 then
        state.ball = None
    end

    if state.cycle_count == 6000 then
        state.finished = true
    end
    if state.cycle_count == game_end_time then
        state.finished = true
    end
end
