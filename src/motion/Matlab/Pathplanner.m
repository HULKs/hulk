%% Tuning parameters
clear all


% Slope of gradient field
goalAlignLimit = 0.5; % Distance until only a alignment to gradient is applied
goalPositionLimit = 0.15; % Distance when no alignment to gradient is applied
maxVelChange = 0.03;
maxDirChange = 0.1;
maxVel = 0.08;
maxDir = 0.2;
gradAlignmentWeight = 1;
obstacleWeight = 1;
obstacleRadius = 0.4;

%% Constants
TO_RAD = pi/180;

% goal
posG = pose2(0, 0, 0);

% poential field
syms x y;
f = -( (x- posG(1,3) ).^2 + (y-posG(2,3) ).^2);
obs = [2 -2];%[1 -1; 2.5 -2.2; 1 1; 2 1; 2 2 ];

% Gradient function
gf = gradient(f, [x,y]);
sym2fun(gf, 'ptGradient');

%% Path Planning
posC = pose2(0,0,0);
speedC = [0.08 ,0.00, 00*pi/180]';
finished = 0;
aligned = 0;
steps= [];
poss= [];

while finished == 0
    % init of variables
    stepC = [0;0;0];
    
    % transformation matrices
    rob2World = posC;
    % robot coordinates
    goal2Rob = rob2World\posG;
    
    % evaluate distance
    dis = sqrt( goal2Rob(1,3)^2 + goal2Rob(2,3)^2);

    grad = goal2Rob(1:2, 3);
    if (norm(grad) ~= 0)
        robGrad = grad/norm(grad);
    else
        robGrad = grad;
    end
    
    % gradient angle
    dirGrad = atan2(robGrad(2),robGrad(1));
    goalGrad = atan2(goal2Rob(2,1), goal2Rob(1,1));
    
    obsGrad = [0;0];
    for i = 1:size(obs,1)
        obsRob = [ [cos(dirGrad) -sin(dirGrad); sin(dirGrad) cos(dirGrad)] rob2World(1:2,3); 0 0 1] \[obs(i,:)';1];
        obsGrad = obsGrad + obstacle(0, 0, obsRob(1:2) , obstacleRadius);
    end
    
    if norm(obsGrad) ~= 0
        obsGrad = obsGrad/norm(obsGrad);
    end
    
    combGrad = robGrad+obstacleWeight*obsGrad;
    if (norm(combGrad) ~= 0)        
        combGrad = combGrad/norm(combGrad);
    end
    
    
    
    %Check Distance to goal
    if (dis > goalAlignLimit)
        % Align to gradient
        gradAlignmentWeight = 1;
        % only align to goal
    elseif dis < goalPositionLimit || (dis <goalAlignLimit && abs(dirGrad) > abs (goalGrad))
        gradAlignmentWeight = 0;
    else
        gradAlignmentWeight = (dis-goalPositionLimit)/...
            (goalAlignLimit-goalPositionLimit);
    end
    
    % Error to gradient
    angleError = (dirGrad*gradAlignmentWeight + goalGrad*(1-gradAlignmentWeight));
    
    turnSpeedDesired = speedC(3)+ maxDirChange*sign(angleError);
    if (abs(turnSpeedDesired) > maxDir)
        turnSpeedDesired = maxDir * sign(angleError);
    end
    
    % braking angle
    brAngle = brakingAngle(speedC(3), maxDirChange);
    brAnglePlus = brakingAngle(turnSpeedDesired, maxDirChange);
    
    turnStepC = 0;
    % Check if braking Angle is less than error
    if abs(brAnglePlus + turnSpeedDesired) < abs(angleError)
        turnStepC = turnSpeedDesired;
        % hold current speed before braking
    elseif abs(brAngle + speedC(3)) < abs(angleError)
        turnStepC = speedC(3);
        % maximal braking
    elseif abs(angleError) > maxDirChange
        turnStepC = speedC(3)-maxDirChange*sign(angleError);
    elseif abs(angleError) < maxDirChange && angleError ~= 0
        turnStepC = angleError;
    elseif angleError == 0
        aligned = 1;
    end
    
    desiredAlignPercentage = abs(turnStepC/maxDir);
    resultingVelPercentage = 1 - desiredAlignPercentage;
    actVelPercentage = norm(speedC(1:2))/maxVel;
    
    % Determine maximal allowed velocity for being able to brake
    
    % current velocity
    curVel = norm(speedC(1:2));    
    if brakingDistance(curVel, maxVelChange) > dis - curVel
        maxVelAllowed = curVel - maxVelChange;
%     elseif brakingDistance(curVel, maxVelChange) < dis -curVel
%         maxVelAllowed = curVel;
    elseif brakingDistance(curVel+maxVelChange, maxVelChange) < dis -abs(curVel+maxVelChange)
        maxVelAllowed = curVel+maxVelChange;
    else
        maxVelAllowed = curVel;
    end
    
    % limitation of speed
    if maxVelAllowed > maxVel
        maxVelAllowed = maxVel;
    % Avoid multiple small steps at end of path
    elseif maxVelAllowed < maxVelChange && dis > maxVelChange
        maxVelAllowed = maxVelChange;
    % if speed is negative or close to 0, apply distance as step size
    % close to 0 may happen when current step equals maxVelChange
    elseif maxVelAllowed <= 1e-10
        maxVelAllowed = dis;
    end
        
    % Having the resulting maximum allowed speed, determine the velocity
    % from the gradient and the velocity part of the maximum motion
    resultingVel = combGrad.*maxVelAllowed*resultingVelPercentage;
    
    % Limit speed if the change of the velocity (for single components) is 
    % too high. The overall speed has been limited already above
    if norm(speedC(1:2) - resultingVel) > maxVelChange
        speedVector = (resultingVel-speedC(1:2));
        speedOffset = speedVector/norm(speedVector)*maxVelChange;
        resultingVel = speedC(1:2)+speedOffset;
    end
    
    % The percentage of the speed vector. We can not limit it further
    % because of change constraints
    resultingVelPercentage = abs(norm(resultingVel)/maxVel);
    
    % Limit turn ratio if velocity can not be limited as required
    if (resultingVelPercentage + desiredAlignPercentage) > 1
        % limit turnStep
        turnStepC = maxDir*(1-resultingVelPercentage)*sign(turnStepC);
    end
    
    % current step
    stepC = [resultingVel; turnStepC];
    speedC = stepC;
    
    % store results for plotting
    steps = [steps stepC];
    poss = [poss [posC(1:2,3); atan2(posC(2,1), posC(1,1))]];
    posC = posC *pose2(stepC(1), stepC(2), stepC(3));
    
    
    % break if dis is small enough (should be really close to 0)
    if dis < maxVelChange/10 && abs(angleError) < maxDirChange/100
        break;
    end
    
    %     visualize;
end

visualize;
