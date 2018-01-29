%% Visualization
figure(1)
[X,Y] = meshgrid(-2:0.2:4, -4:0.2:2);
G = subs(gf, {x,y}, {X,Y});

GXO = zeros(size(X));
GYO = GXO;

for k = 1:size(obs,1)
    for i = 1:size(X)
        for j = 1:size(Y)
            GObs = obstacle(X(i,j), Y(i,j), obs(k,:)', obstacleRadius);
            GXO(i,j) = GXO(i,j) + GObs(1);
            GYO(i,j) = GYO(i,j) + GObs(2);
        end
    end
end

SCALEO = sqrt(GXO.^2 + GYO.^2);
GXO = GXO./SCALEO;
GYO = GYO./SCALEO;

GXF = G(1:size(X,1),:);
GYF = G(size(X,1)+1:end,:);

SCALEF = sqrt(GXF.^2 + GYF.^2);
GXF = GXF./SCALEF;
GYF = GYF./SCALEF;

GX = GXF + obstacleWeight*GXO;
GY = GYF + obstacleWeight*GYO;

SCALECOMB = sqrt(GX.^2 + GY.^2);
GX = GX./SCALECOMB;
GY = GY./SCALECOMB;


quiver(X,Y,GXO, GYO);
hold all;
quiver(poss(1,:), poss(2,:), cos(poss(3,:)), sin(poss(3,:)));
plot(poss(1,:), poss(2,:), '-r', 'LineWidth',2);
plot(obs(:,1), obs(:,2), 'o', 'MarkerSize',30);
plot(posG(1,3),posG(2,3), 'xk', 'MarkerSize', 10, 'LineWidth', 3);
quiver(posG(1,3), posG(2,3), posG(1,1), posG(2,1));
grid on;
hold off;