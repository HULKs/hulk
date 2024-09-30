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
        state.game_controller_state.game_state = "Ready"
        state.game_controller_state.kicking_team = "Hulks"
    end

    if state.cycle_count == 1600 then
        state.game_controller_state.game_state = "Set"
    end

    if state.cycle_count == 1700 then
        state.game_controller_state.game_state = "Playing"
    end

    if state.cycle_count == 1750 then
        penalize(1);
        state.game_controller_state.penalties.one = {
            Manual = {
                remaining = {
                    nanos = 0,
                    secs = 50
                },
            }
        };
        set_robot_pose(1, { -3.2, 3 }, -1.5707963267948966);
    end

    if state.cycle_count == 2000 then
        penalize(2);
        state.game_controller_state.penalties.two = {
            Manual = {
                remaining = {
                    nanos = 0,
                    secs = 50
                },
            }
        };
        set_robot_pose(2, { -3.1, 3 }, -1.5707963267948966);
    end

    if state.cycle_count == 3550 then
        state.ball = {
            position = { 0.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end
    
    if state.cycle_count == 2550 then
        state.ball = {
            position = { 0.0, 0.0 },
            velocity = { 0.0, 0.0 },
        }
    end

    if state.cycle_count == 4500 then
        unpenalize(1);
        state.game_controller_state.penalties.one = nil;
    end

    if state.cycle_count == 8000 then
        state.finished = true
    end
end
