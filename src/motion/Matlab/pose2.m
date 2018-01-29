function p2 = pose2( x,y,alpha )
%POSE2 Summary of this function goes here
%   Detailed explanation goes here


p2 = [cos(alpha) -sin(alpha) x;
        sin(alpha)  cos(alpha) y;
        0 0 1];


end

