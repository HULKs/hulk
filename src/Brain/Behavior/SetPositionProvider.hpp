#pragma once

#include <vector>

#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SetPosition.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"


class Brain;

class SetPositionProvider : public Module<SetPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"SetPositionProvider"};

  explicit SetPositionProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  /**
   * @brief getPermutationValue computes a value for a given position assignment (the less the
   * better)
   * @param perm the permutation (assignment of players to positions) that is to be checked
   * @param positions the positions that are to be assigned
   * @param remainingTeamPlayers the positions of the team members which are neither keeper nor
   * kickoff striker
   * @param signY the sign by which the y coordinates of the positions are multiplied (for mirroring
   * on the x-axis)
   */
  float getPermutationValue(const std::vector<unsigned int>& perm, const VecVector2f& positions,
                            const VecVector2f& remainingTeamPlayers, float signY) const;

  /**
   * @brief roleIsCompatibleWithPosition checks whether a certain role may occupy a certain set
   * position
   * @param role the role which is to check
   * @param posIndex the index of the position in the offensive or defensive positions
   * @return true iff the role is compatible with the position
   */
  bool roleIsCompatibleWithPosition(PlayingRole role, unsigned int posIndex) const;

  const Parameter<Vector2f> keeperPosition_;
  const Parameter<VecVector2f> defensivePositions_;
  const Parameter<VecVector2f> offensivePositions_;
  const Parameter<VecVector2f> defensivePenaltyKickPositions_;
  const Parameter<VecVector2f> offensivePenaltyKickPositions_;
  const Parameter<bool> considerRole_;
  const Parameter<bool> enableDribbleAroundOpponentAtKickoff_;
  Parameter<float> dribbleAngle_;
  float kickoffDribbleSign_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamPlayers> teamPlayers_;
  Production<SetPosition> setPosition_;
};
