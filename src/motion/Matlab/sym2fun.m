
function sym2fun(gradient, str)
 
up = pwd;

%up = [up(1:end-1) '\SystemFunctions\'];

% Create a file str.m
fid = fopen([up '/' str '.m'],'w');
 
% Comment : date
date = fix(clock);
fprintf(fid,'%% %d-%d-%d %d:%d:%d\n\n',date);
 
% creates the function header
% fprintf(fid,'function out = %s(%s)\n\n',str,findsym(A));

fprintf(fid,'function out = %s(%s)\n\n',str, 'x, y');

% creates the output
fprintf(fid,'out = [... \n');

fprintf(fid, [char(gradient(1)), ';\n']);
fprintf(fid, [char(gradient(2)), '];\n']);

fclose(fid);