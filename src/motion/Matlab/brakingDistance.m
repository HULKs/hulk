function out = brakingDistance( currentSpeed, maxChange )
%BRAKINGANGLE Summary of this function goes here
%   Detailed explanation goes here

if currentSpeed > 0
speed = currentSpeed;
out = 0;

steps = floor(speed/maxChange);

for i = 1:steps
    
    speed = speed - maxChange * sign(speed);    
    out = out +speed;
end

out = out + speed;
else
    out = 0;

end

