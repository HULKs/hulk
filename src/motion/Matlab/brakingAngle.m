function out = brakingAngle( currentSpeed, maxChange )
%BRAKINGANGLE Summary of this function goes here
%   Detailed explanation goes here

speed = currentSpeed;
out = 0;

steps = floor(speed/maxChange)*sign(currentSpeed);

for i = 1:steps
    
    speed = speed - maxChange * sign(speed);    
    out = out +speed;
end

out = out +speed;

end

