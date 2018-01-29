function val = obstacle( x, y ,m,r )
%OBSTACLE Summary of this function goes here
%   Detailed explanation goes here

if(norm([x;y] - m) < r && (x-m(1)) > -r/4)
    s = [x;y]-m;
    s = s/norm(s);
    
    if m(2) > 0
        fac = 1;
    else
        fac = -1;
    end
    alpha = fac*60*pi/180;
    
    val= [cos(alpha) -sin(alpha); sin(alpha)  cos(alpha)] * s;
       
else
    val = [0;0];
end

end

