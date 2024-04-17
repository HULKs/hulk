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

function on_cycle()
    if state.ball == nil and state.cycle_count % 1000 == 0 then
        print(inspect(state))
        state.ball = {
            position = { 0.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
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
                kick_off = false
            }
        }
    end

    if state.cycle_count == 1750 then
        penalize(1);
        state.filtered_game_controller_state.penalties.one = {
            Manual = {
                remaining = {
                    nanos = 0,
                    secs = 5
                },
            }
        };
        set_robot_pose(1, { -3.2, 3 }, -1.5707963267948966);
    end

    if state.cycle_count == 2150 then
        unpenalize(1);
        state.filtered_game_controller_state.penalties.one = nil;
    end

    if state.cycle_count == 3550 then
        state.ball = {
            position = { 0.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 3600 then
        penalize(1);
        state.filtered_game_controller_state.penalties.one = {
            Manual = {
                remaining = {
                    nanos = 0,
                    secs = 5
                },
            }
        };
        set_robot_pose(1, { -3.2, 3 }, -1.5707963267948966);
    end

    if state.cycle_count == 3900 then
        unpenalize(1);
        state.filtered_game_controller_state.penalties.one = nil;
    end

    if state.cycle_count == 4650 then
        state.ball = {
            position = { -4.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == game_end_time then
        state.finished = true
    end
end
