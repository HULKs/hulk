#include <cmath>

#include <Eigen/Dense>

#include "Modules/NaoProvider.h"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "BallSearchPositionProvider.hpp"


BallSearchPositionProvider::BallSearchPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager, "BallSearchPositionProvider")
  , gameControllerState_(*this)
  , playerConfiguration_(*this)
  , playingRoles_(*this)
  , teamPlayers_(*this)
  , ballState_(*this)
  , robotPosition_(*this)
  , bodyPose_(*this)
  , teamBallModel_(*this)
  , fieldDimensions_(*this)
  , jointSensorData_(*this)
  , cycleInfo_(*this)
  , rowsCount_(*this, "rows", [this] { rebuildProbabilityMap(); })
  , colsCount_(*this, "cols", [this] { rebuildProbabilityMap(); })
  , minBallDetectionRange_(*this, "minBallDetectionRange")
  , maxBallDetectionRange_(*this, "maxBallDetectionRange", [this] { maxBallDetectionRangeSquared_ = maxBallDetectionRange_() * maxBallDetectionRange_(); })
  , maxBallAge_(*this, "maxBallAge", [] {})
  , fovAngle_(*this, "fovAngle", [this] { fovAngle_() *= TO_RAD; })
  , minProbabilityToStartSearch_(*this, "minProbabilityToStartSearch")
  , minProbabilityToForceSearch_(*this, "minProbabilityToForceSearch")
  , minAgeToStartSearch_(*this, "minAgeToStartSearch")
  , minAgeToForceSearch_(*this, "minAgeToForceSearch")
  , convolutionKernelCoreWeight_(*this, "convolutionKernelCoreWeight")
  , minDistanceBetweenSearchPositions_(*this, "minDistanceBetweenSearchPositions")
  , confidentBallMultiplier_(*this, "confidentBallMultiplier")
  , unconfidentBallMultiplier_(*this, "unconfidentBallMultiplier")
  , searchPosition_(*this)
  , fieldLength_(fieldDimensions_->fieldLength)
  , fieldWidth_(fieldDimensions_->fieldWidth)
  , ballSeenThisCycle_(false)
{
  fovAngle_() *= TO_RAD; // Obviously.
  rebuildProbabilityMap();
}

void BallSearchPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");

  assert(minAgeToStartSearch_() <= minAgeToForceSearch_());
  assert(minProbabilityToStartSearch_() <= minProbabilityToForceSearch_());

  // Updating the map while not playing may make things worse.
  if (gameControllerState_->state == GameState::PLAYING)
  {
    activePlayers_.clear();
    explorers_.clear();
    playersToUpdate_.clear();
    // create a list of all active players (those who are not penalized)
    if (gameControllerState_->penalty == Penalty::NONE)
    {
      Player ownPlayer;
      ownPlayer.playerNumber = playerConfiguration_->playerNumber;
      ownPlayer.isHULK = true;
      ownPlayer.pose = robotPosition_->pose;
      ownPlayer.isPoseValid = robotPosition_->valid;
      ownPlayer.ballPosition = robotPosition_->robotToField(ballState_->position);
      ownPlayer.ballAge = ballState_->age;
      ownPlayer.isBallConfident = ballState_->confident;
      ownPlayer.fallen = bodyPose_->fallen;
      ownPlayer.penalized = false;
      ownPlayer.headYaw = jointSensorData_->angles[keys::joints::HEAD_YAW];
      ownPlayer.currentSearchPosition = finalSearchPose_.position;
      ownPlayer.isSearchPositionImportant = isCellImportant(toCell(finalSearchPose_.position));
      ownPlayer.isSearchPositionOutdated = !isCellCandidate(toCell(finalSearchPose_.position));

      activePlayers_.push_back(ownPlayer);
    }
    for (auto teamPlayer = teamPlayers_->players.begin(); teamPlayer != teamPlayers_->players.end(); teamPlayer++)
    {
      Player player;
      player.playerNumber = teamPlayer->playerNumber;
      player.isHULK = teamPlayer->isHULK;
      player.pose = teamPlayer->pose;
      // Values as seen in SPLMessageTransmitter.
      player.isPoseValid = teamPlayer->currentPositionConfidence >= 50; //&& teamPlayer->currentSideConfidence >= 0;
      player.ballPosition = teamPlayer->pose * teamPlayer->ballPosition;
      player.ballAge = cycleInfo_->getTimeDiff(teamPlayer->timeWhenBallWasSeen);
      player.isBallConfident = true;
      player.fallen = teamPlayer->fallen;
      player.penalized = teamPlayer->penalized;
      player.headYaw = teamPlayer->headYaw;
      Vector2f fakedSerachPosition = teamPlayer->pose * Vector2f(1.5f, 0.f);
      player.currentSearchPosition = teamPlayer->isHULK ? teamPlayer->currentSearchPosition : fakedSerachPosition;
      player.isSearchPositionImportant = isCellImportant(toCell(teamPlayer->currentSearchPosition));
      player.isSearchPositionOutdated = !isCellCandidate(toCell(teamPlayer->currentSearchPosition));

      activePlayers_.push_back(player);
    }
    std::sort(activePlayers_.begin(), activePlayers_.end(), [](const Player& p1, const Player& p2) { return p1.playerNumber < p2.playerNumber; });

    // find players that are searching at a position that is not interesting anymore. Also create a list with all players
    // that can be assigned to a search position (this excludes keeper and players of other teams.
    for (Player& player : activePlayers_)
    {
      if (player.playerNumber != 1 && player.isHULK)
      {
        if (player.isSearchPositionOutdated)
        {
          playersToUpdate_.push_back(&player);
        }
        explorers_.push_back(&player);
      }
    }

    updateMap();

    // update the 'assigned' state of all cells that are already being searched by a robot.
    for (Player& player : activePlayers_)
    {
      if (!player.isSearchPositionOutdated)
      {
        ProbabilityCell& cell = toCell(player.currentSearchPosition);
        cell.isAssigned = true;
        setCellsInsideRadius(cell, minDistanceBetweenSearchPositions_(),
                             [this, cell](int dx, int dy) { probabilityMap_[cell.indices.x() + dx][cell.indices.y() + dy].isAssigned = true; });
      }
    }

    if (activePlayers_.size() > 0)
    {
      generateSearchCandidates();
      updateSearchPositions();
    }

    // Generate a pose for the own robot for looking at the assigned search position.
    // This pose needs to be in a certain radius around the search pose (also not too close).
    Vector2f relCellPosition = searchPosition_->searchPosition - robotPosition_->pose.position;
    if (relCellPosition.norm() > minBallDetectionRange_())
    {
      const float relativeCellAngle = static_cast<float>(atan2(relCellPosition.y(), relCellPosition.x()));
      if (relCellPosition.norm() > maxBallDetectionRange_() / 2.f)
      {
        // The robot is too far away from the search cell. Move towards it.
        relCellPosition = relCellPosition.normalized() * (relCellPosition.norm() - (maxBallDetectionRange_() / 2.f));
      }
      else
      {
        // The robot is close enough to the search cell. Keep current distance.
        // TODO: This seems strange
        relCellPosition = Vector2f::Zero();
      }
      searchPosition_->pose = Pose(robotPosition_->pose.position + relCellPosition, relativeCellAngle);
    }
    else
    {
      // the robot is too close to the target position. Move away.
      searchPosition_->pose = Pose(robotPosition_->pose.position.x() + relCellPosition.x() - (minBallDetectionRange_() * 1.5f),
                                   robotPosition_->pose.position.y() + relCellPosition.y());
    }
  }
  else
  {
    // While not playing the search pose is set to a position near the center of the field.
    searchPosition_->pose = Pose(-0.5f, 0.f);
  }

  sendDebug();
}

void BallSearchPositionProvider::updateMap()
{
  ballSeenThisCycle_ = false;

  // update map with all player data available.
  for (auto& player : activePlayers_)
  {
    if (!player.fallen)
    {
      updateWithRobot(player.pose, player.isPoseValid, player.ballPosition, player.ballAge, player.isBallConfident, player.headYaw);
    }
  }

  // set outer cells probabilities manually (to the nearest cell's probability).
  for (int x = 1; x < colsCount_() - 1; x++)
  {
    probabilityMap_[x][0].probability = probabilityMap_[x][1].probability;
    probabilityMap_[x][rowsCount_() - 1].probability = probabilityMap_[x][rowsCount_() - 2].probability;
  }
  for (int y = 1; y < rowsCount_() - 1; y++)
  {
    probabilityMap_[0][y].probability = probabilityMap_[1][y].probability;
    probabilityMap_[colsCount_() - 1][y].probability = probabilityMap_[colsCount_() - 2][y].probability;
  }
  probabilityMap_[0][0].probability = probabilityMap_[1][1].probability;
  probabilityMap_[0][rowsCount_() - 1].probability = probabilityMap_[1][rowsCount_() - 2].probability;
  probabilityMap_[colsCount_() - 1][0].probability = probabilityMap_[colsCount_() - 2][1].probability;
  probabilityMap_[colsCount_() - 1][rowsCount_() - 1].probability = probabilityMap_[colsCount_() - 2][rowsCount_() - 2].probability;

  // Backup the probabilities from the last cycle and delete isSearchPositionCandidate status.
  for (int x = 0; x < colsCount_(); x++)
  {
    for (int y = 0; y < rowsCount_(); y++)
    {
      // probabilityMap_[x][y].probability = probabilityMap_[x][y].probability == 0 ? 0.0001f : probabilityMap_[x][y].probability;
      probabilityMap_[x][y].oldProbability = probabilityMap_[x][y].probability;
      probabilityMap_[x][y].isSearchPositionCandidate = false;
      probabilityMap_[x][y].isCloseToSearchPositionCandidate = false;
      probabilityMap_[x][y].isAssigned = false;
    }
  }

  // convolve with {{1,1,1},{1,x,1},{1,1,1}} so that a single, highly probable cell will spread its probability to
  // the neighbours with time. Note that the resulting probability is only saved as the new value if it is greater than before.
  for (int x = 1; x < colsCount_() - 1; x++)
  {
    for (int y = 1; y < rowsCount_() - 1; y++)
    {
      float result = probabilityMap_[x - 1][y - 1].oldProbability + probabilityMap_[x][y - 1].oldProbability + probabilityMap_[x + 1][y - 1].oldProbability +
                     probabilityMap_[x - 1][y].oldProbability + static_cast<float>(convolutionKernelCoreWeight_()) * probabilityMap_[x][y].oldProbability +
                     probabilityMap_[x + 1][y].oldProbability + probabilityMap_[x - 1][y + 1].oldProbability + probabilityMap_[x][y + 1].oldProbability +
                     probabilityMap_[x + 1][y + 1].oldProbability;
      result *= 1.f / (convolutionKernelCoreWeight_() + 8);

      probabilityMap_[x][y].probability = probabilityMap_[x][y].oldProbability < result ? result : probabilityMap_[x][y].oldProbability;
    }
  }

  // Non outer cells: calculate weight sum.
  float weightSum = 0.f; // the weight sum for normalization.
  for (int x = 1; x < colsCount_() - 1; x++)
  {
    for (int y = 1; y < rowsCount_() - 1; y++)
    {
      weightSum += probabilityMap_[x][y].probability;
    }
  }
  assert(weightSum > 0.f);

  // Non outer cells: do aging and normalization
  for (int x = 1; x < colsCount_() - 1; x++)
  {
    for (int y = 1; y < rowsCount_() - 1; y++)
    {
      probabilityMap_[x][y].probability /= weightSum; // normalize this cell.
      probabilityMap_[x][y].age += 1;
    }
  }
}

void BallSearchPositionProvider::generateSearchCandidates()
{
  lastSearchPose_ = finalSearchPose_;
  searchCellCandidates_.clear();
  importantSearchCells_.clear();
  searchPosition_->reset();

  // add most probable cells (if any) to the searchPositions
  probabilityList_.sort(isCellMoreProbable);
  for (std::list<ProbabilityCell*>::reverse_iterator rit = probabilityList_.rbegin(); rit != probabilityList_.rend(); rit++)
  {
    if ((*rit)->probability < minProbabilityToStartSearch_())
    {
      break;
    }
    if ((*rit)->isCloseToSearchPositionCandidate)
    {
      continue;
    }
    if ((*rit)->probability > minProbabilityToForceSearch_())
    {
      importantSearchCells_.push_back(*rit);
      searchCellCandidates_.push_back(*rit);
      for (Player& player : activePlayers_)
      {
        if (&(toCell(player.currentSearchPosition)) == *rit)
        {
          player.isSearchPositionImportant = true;
          (*rit)->isAssigned = true;
          setCellsInsideRadius(*(*rit), minDistanceBetweenSearchPositions_(),
                               [this, rit](int dx, int dy) { probabilityMap_[(*rit)->indices.x() + dx][(*rit)->indices.y() + dy].isAssigned = true; });
        }
      }
      setCellsInsideRadius(*(*rit), minDistanceBetweenSearchPositions_(), [this, rit](int dx, int dy) {
        probabilityMap_[(*rit)->indices.x() + dx][(*rit)->indices.y() + dy].isCloseToSearchPositionCandidate = true;
      });
    }
    else
    {
      searchCellCandidates_.push_back(*rit);
      (*rit)->isSearchPositionCandidate = true;
      setCellsInsideRadius(*(*rit), minDistanceBetweenSearchPositions_(), [this, rit](int dx, int dy) {
        probabilityMap_[(*rit)->indices.x() + dx][(*rit)->indices.y() + dy].isCloseToSearchPositionCandidate = true;
      });
    }
  }

  // add oldest cells (if any) to the searchPositions
  probabilityList_.sort(isCellOlder);
  for (std::list<ProbabilityCell*>::reverse_iterator rit = probabilityList_.rbegin(); rit != probabilityList_.rend(); rit++)
  {
    if ((*rit)->age < static_cast<uint32_t>(minAgeToStartSearch_()))
    {
      break;
    }
    if ((*rit)->isCloseToSearchPositionCandidate)
    {
      continue;
    }
    if ((*rit)->age > static_cast<uint32_t>(minAgeToForceSearch_()))
    {
      importantSearchCells_.push_back(*rit);
      searchCellCandidates_.push_back(*rit);
      for (Player& player : activePlayers_)
      {
        if (&(toCell(player.currentSearchPosition)) == *rit)
        {
          player.isSearchPositionImportant = true;
          (*rit)->isAssigned = true;
          setCellsInsideRadius(*(*rit), minDistanceBetweenSearchPositions_(),
                               [this, rit](int dx, int dy) { probabilityMap_[(*rit)->indices.x() + dx][(*rit)->indices.y() + dy].isAssigned = true; });
        }
      }
      setCellsInsideRadius(*(*rit), minDistanceBetweenSearchPositions_(), [this, rit](int dx, int dy) {
        probabilityMap_[(*rit)->indices.x() + dx][(*rit)->indices.y() + dy].isCloseToSearchPositionCandidate = true;
      });
    }
    else
    {
      searchCellCandidates_.push_back(*rit);
      (*rit)->isSearchPositionCandidate = true;
      setCellsInsideRadius(*(*rit), minDistanceBetweenSearchPositions_(), [this, rit](int dx, int dy) {
        probabilityMap_[(*rit)->indices.x() + dx][(*rit)->indices.y() + dy].isCloseToSearchPositionCandidate = true;
      });
    }
  }
}

void BallSearchPositionProvider::updateWithRobot(const Pose& pose, bool isPoseValid, const Vector2f& ballPosition, const float ballAge, bool isBallConfident,
                                                 const float headYaw)
{
  // If the robot is not sure about its position, there should not be an update (because isCellInFOV will not work).
  if (!isPoseValid)
  {
    // If the pose is not validated it is not a good idea to include the data sent by this robot.
    return;
  }
  if (isBallConfident)
  {
    ballSeenThisCycle_ = true;
    // A ball was detected. Update probability cell the ball is in and increase adjacent cell's prob.
    ProbabilityCell& cellWithBall = toCell(ballPosition);
    cellWithBall.probability = std::max(0.01f, cellWithBall.probability * confidentBallMultiplier_());
    cellWithBall.age = 0;
  }
  else
  {
    ballSeenThisCycle_ = false;

    if (ballAge < maxBallAge_())
    {
      ProbabilityCell& cellWithBall = toCell(ballPosition);
      cellWithBall.probability = std::max(0.01f, cellWithBall.probability * unconfidentBallMultiplier_());
    }
  }

  // Decrease probability of all cells in FOV (including the cell containing the ball if there was any).
  for (int x = 1; x < colsCount_() - 1; x++)
  {
    for (int y = 1; y < rowsCount_() - 1; y++)
    {
      if (isCellInFOV(pose, headYaw, probabilityMap_[x][y]))
      {
        // Todo: Remove this magic numbers.
        probabilityMap_[x][y].probability *= (ballAge < maxBallAge_()) ? 0.99f : 0.98f; // Reduce probability a bit.
        probabilityMap_[x][y].age = 0;
      }
    }
  }
}

void BallSearchPositionProvider::updateSearchPositions()
{
  switch (explorers_.size())
  {
    case 0:
      // nothing to do here. There is no player on the field.
      break;
    case 1:
      // do not update the search position if it the robot already searches an important cell.
      if (!(explorers_[0]->isSearchPositionImportant))
      {
        // let the only player on the field search the most important search cell if there is any. Else the closest one.
        if (importantSearchCells_.size() > 0)
        {
          explorers_[0]->currentSearchPosition = importantSearchCells_[0]->position;
        }
        else
        {
          if (searchCellCandidates_.size() == 0)
          {
            // TODO: send playing position as suggestion.
            break;
          }
          if (explorers_[0]->isSearchPositionOutdated)
          {
            float bestTimeToReachCell = std::numeric_limits<float>::max();
            for (ProbabilityCell* cell : searchCellCandidates_)
            {
              float time = timeToReachCell(*(explorers_[0]), *cell);
              if (time < bestTimeToReachCell)
              {
                bestTimeToReachCell = time;
                explorers_[0]->currentSearchPosition = cell->position;
              }
            }
          }
        }
      }
      break;
    default:
      for (ProbabilityCell* cell : importantSearchCells_)
      {
        if (!cell->isAssigned)
        {
          float minWeight = std::numeric_limits<float>::max();
          Player* playerToAssign = nullptr;
          for (Player* player : explorers_)
          {
            if (!player->isSearchPositionImportant)
            {
              float time = timeToReachCell(*player, *cell);
              if (time < minWeight)
              {
                minWeight = time;
                playerToAssign = player;
              }
            }
            else
            {
              // Player is already searching an important search cell. Do not disturb!
            }
          }
          if (playerToAssign != nullptr)
          {
            // assign the search cell to the nearest player.
            playerToAssign->currentSearchPosition = cell->position;
            playerToAssign->isSearchPositionImportant = true;
            playerToAssign->isSearchPositionOutdated = false;
            cell->isAssigned = true;
            setCellsInsideRadius(*cell, minDistanceBetweenSearchPositions_(),
                                 [this, cell](int dx, int dy) { probabilityMap_[cell->indices.x() + dx][cell->indices.y() + dy].isAssigned = true; });
          }
        }
        else
        {
          // skip. This cell is already assigned.
        }
      }
      uint8_t unassignedRobots = 0;
      for (Player* player : playersToUpdate_)
      {
        if (!player->isSearchPositionImportant)
        {
          unassignedRobots++;
        }
      }
      if (unassignedRobots == 0)
      {
        break;
      }
      for (ProbabilityCell* cell : searchCellCandidates_)
      {
        if (!cell->isAssigned)
        {
          float minWeight = std::numeric_limits<float>::max();
          Player* playerToAssign = nullptr;
          for (Player* player : playersToUpdate_)
          {
            if (!player->isSearchPositionImportant && player->isSearchPositionOutdated)
            {
              float time = timeToReachCell(*player, *cell);
              if (time < minWeight)
              {
                minWeight = time;
                playerToAssign = player;
              }
            }
          }
          if (playerToAssign != nullptr)
          {
            playerToAssign->currentSearchPosition = cell->position;
            playerToAssign->isSearchPositionOutdated = false;
            cell->isAssigned = true;
            setCellsInsideRadius(*cell, minDistanceBetweenSearchPositions_(),
                                 [this, cell](int dx, int dy) { probabilityMap_[cell->indices.x() + dx][cell->indices.y() + dy].isAssigned = true; });
          }
        }
      }
      // DEFAULT CASE END
  }

  // copy the suggested search positions to the production.
  for (Player player : activePlayers_)
  {
    searchPosition_->suggestedSearchPositions[player.playerNumber - 1] = player.currentSearchPosition;
  }

  // get the lowest player number (the player with the lowest player number will tell everyone else where to search)
  uint8_t lowestPlayerNumber = static_cast<uint8_t>(playerConfiguration_->playerNumber);
  const TeamPlayer* currentKing = nullptr;
  for (std::list<TeamPlayer>::const_iterator teamPlayer = teamPlayers_->players.begin(); teamPlayer != teamPlayers_->players.end(); teamPlayer++)
  {
    if (teamPlayer->playerNumber < lowestPlayerNumber)
    {
      lowestPlayerNumber = static_cast<uint8_t>(teamPlayer->playerNumber);
      currentKing = &(*teamPlayer);
    }
  }

  // accept the suggestion of the player with the lowest player number (this might be the robot itself).
  if (lowestPlayerNumber == playerConfiguration_->playerNumber)
  {
    searchPosition_->searchPosition = searchPosition_->suggestedSearchPositions[playerConfiguration_->playerNumber - 1];
  }
  else
  {
    if (currentKing != nullptr && currentKing->suggestedSearchPositions.size() >= playerConfiguration_->playerNumber)
    {
      searchPosition_->searchPosition = currentKing->suggestedSearchPositions[playerConfiguration_->playerNumber - 1];
    }
    else
    {
      searchPosition_->searchPosition = searchPosition_->suggestedSearchPositions[playerConfiguration_->playerNumber - 1];
    }
  }

  finalSearchPose_ = searchPosition_->searchPosition;
}


bool BallSearchPositionProvider::isCellInBallDetectionRange(const Pose& pose, const ProbabilityCell& cell)
{
  return (cell.position - pose.position).squaredNorm() < maxBallDetectionRangeSquared_;
}

bool BallSearchPositionProvider::isCellInFOV(const Pose& pose, const float headYaw, const ProbabilityCell& cell)
{
  // relative cell in polar coordinates:
  Vector2f relCellPosition = cell.position - pose.position;
  if (relCellPosition.squaredNorm() < maxBallDetectionRangeSquared_)
  {
    const float relativeCellAngle = static_cast<float>(atan2(relCellPosition.y(), relCellPosition.x()));
    const float angleToHeadX = Angle::angleDiff(relativeCellAngle, headYaw + pose.orientation);
    // Cell is in the radius => cell may be in FOV
    if (std::abs(angleToHeadX) < fovAngle_() * 0.5f)
    {
      // Cell is between start and end vector of FOV => Cell is in FOV.
      return true;
    }
  }
  return false;
}

bool BallSearchPositionProvider::isCellMoreProbable(const ProbabilityCell* first, const ProbabilityCell* second)
{
  return first->probability < second->probability;
}

bool BallSearchPositionProvider::isCellOlder(const ProbabilityCell* first, const ProbabilityCell* second)
{
  return first->age < second->age;
}

int BallSearchPositionProvider::getCosts(const Pose& currentPose, const Vector2f& currentSearchPosition,
                                         const BallSearchPositionProvider::ProbabilityCell& cellToExplore)
{
  float distanceToCurrentSearchPosition = (currentPose.position - currentSearchPosition).norm();
  float distanceToNewCell = (currentPose.position - cellToExplore.position).norm();

  return static_cast<int>(distanceToCurrentSearchPosition * distanceToNewCell * (1.f / cellToExplore.probability));
}

float BallSearchPositionProvider::timeToReachCell(Player& player, ProbabilityCell& cell)
{
  const Vector2f relCellPosition = cell.position - player.pose.position;
  // TODO: Is 15cm per second a good approximation?
  const float walkTimeDistance = relCellPosition.norm() / 0.18f;
  // TODO: Is 10s per 180° a good approximation?
  const float cellOrientation = std::atan2(relCellPosition.y(), relCellPosition.x());
  const float rotateTimeDistance = Angle::angleDiff(cellOrientation, player.pose.orientation) * 10.f / M_PI;
  // TODO: Is 10s a good approximation?
  const float fallenPenalty = player.fallen ? 10.0f : 0.0f;

  return walkTimeDistance + rotateTimeDistance + fallenPenalty;
}

void BallSearchPositionProvider::sendDebug()
{
  debug().update(mount_ + ".ballSearchProbabilityMap", probabilityMap_);
  debug().update(mount_ + ".ballSearchPose", searchPosition_->pose);
  debug().update(mount_ + ".ballAgeTemp", ballState_->age);
  if (debug().isSubscribed(mount_ + "potentialSearchPoses"))
  {
    cellsToSend_.clear();
    for (std::vector<ProbabilityCell*>::const_iterator it = searchCellCandidates_.begin(); it != searchCellCandidates_.end(); it++)
    {
      cellsToSend_.emplace_back(*(*it));
    }

    debug().update(mount_ + ".potentialSearchPoses", cellsToSend_);
  }
}

BallSearchPositionProvider::ProbabilityCell& BallSearchPositionProvider::toCell(const Vector2f& position)
{
  int x = std::max(1, static_cast<int>((position.x() + fieldLength_ / 2.f) / cellLength_) + 1);
  int y = std::max(1, static_cast<int>((position.y() + fieldWidth_ / 2.f) / cellHeight_) + 1);

  // Cell index can not be more than number of cols or rows minus 1. (Preventing seg faults)
  x = std::min(colsCount_() - 2, x);
  y = std::min(rowsCount_() - 2, y);

  return probabilityMap_[x][y];
}

void BallSearchPositionProvider::rebuildProbabilityMap()
{
  cellHeight_ = fieldWidth_ / (float)(rowsCount_() - 2);
  cellLength_ = fieldLength_ / (float)(colsCount_() - 2);
  maxBallDetectionRangeSquared_ = maxBallDetectionRange_() * maxBallDetectionRange_();

  // Todo: Really clears all memory allocated?
  probabilityMap_.clear();

  probabilityMap_.reserve(colsCount_());
  for (int x = 0; x < colsCount_(); x++)
  {
    std::vector<ProbabilityCell> probCells;
    probCells.reserve(rowsCount_());
    for (int y = 0; y < rowsCount_(); y++)
    {
      ProbabilityCell probCell;
      probCell.reset();
      // Set x&y to center of the current index
      probCell.position.x() = ((float)(x - 1) * (fieldLength_ / (colsCount_() - 2)) + 0.5f * cellLength_) - fieldLength_ / 2.f;
      probCell.position.y() = ((float)(y - 1) * (fieldWidth_ / (rowsCount_() - 2)) + 0.5f * cellHeight_) - fieldWidth_ / 2.f;
      probCell.indices.x() = x;
      probCell.indices.y() = y;
      probCell.isAssigned = false;
      probCell.isCloseToSearchPositionCandidate = false;
      probCell.probability = 1.f / static_cast<float>((colsCount_() * rowsCount_()));
      probCell.oldProbability = probCell.probability;
      probCell.age = static_cast<uint32_t>(minAgeToStartSearch_());
      probCells.push_back(probCell);
    }
    probabilityMap_.push_back(probCells);
  }
  for (int x = 1; x < colsCount_() - 1; x++)
  {
    for (int y = 1; y < rowsCount_() - 1; y++)
    {
      probabilityList_.push_back(&(probabilityMap_[x][y]));
    }
  }
  searchPosition_->reset();
  searchPosition_->pose = {-0.5f, 0.f, 0.f};
  searchPosition_->suggestedSearchPositions.resize(MAX_NUM_PLAYERS);
  for (unsigned int i = 0; i < searchPosition_->suggestedSearchPositions.size(); i++)
  {
    searchPosition_->suggestedSearchPositions[i] = Vector2f(0.f, 0.f);
  }
  finalSearchPose_ = {-0.5f, 0.f};
  dummyCell_.position.x() = -0.5f;
  dummyCell_.position.y() = 0.f;
  dummyCell_.probability = 0.f;
  dummyCell_.age = 0;
  dummyCell_.indices.x() = 0;
  dummyCell_.indices.y() = 0;
}
