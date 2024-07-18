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
        state.game_controller_state.kicking_team = "Opponent"
    end

    if state.cycle_count == 1600 then
        state.game_controller_state.game_state = "Set"
    end

    if state.cycle_count == 1700 then
        -- TODO: simulate whistle
        whistle(1)
        whistle(2)
        whistle(3)
        whistle(4)
        whistle(5)
        whistle(6)
        whistle(7)
    end

    -- 10 frames in simulator are counted as 10 seconds in the game_controller_state_filter and thus correspond to the opponent kick off time
    if state.cycle_count >= 1700 and state.cycle_count < 2200 then
        for i = 1, 7 do
            is_in_opponent_half = get_robot_pose_x(i) >= 0.0
            if is_in_opponent_half then
                -- error("Illegal Position in Kickoff Playing")
            end
            is_in_center_circle = math.sqrt(get_robot_pose_x(i) ^ 2 + get_robot_pose_y(i) ^ 2) <= 0.75
            if is_in_center_circle then
                -- error("Illegal Position in Kickoff Playing")
            end
        end
    end

    if state.cycle_count == 2200 then
        state.game_controller_state.game_state = "Playing"
    end

    if state.cycle_count == 15000 then
        state.finished = true
    end

    if state.cycle_count == game_end_time then
        state.finished = true
    end
end
