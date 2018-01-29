#include "Tools/Chronometer.hpp"

#include "LandmarkFilter.hpp"

LandmarkFilter::LandmarkFilter(const ModuleManagerInterface& manager)
  : Module(manager, "LandmarkFilter")
  , bufferGoalPosts_(*this, "bufferGoalPosts", [] {})
  , maxGoalPostDistanceDeviation_(*this, "maxGoalPostDistanceDeviation", [] {})
  , maxGoalPostAge_(*this, "maxGoalPostAge", [] {})
  , goalPostAssociationRadius_(*this, "goalPostAssociationRadius", [] {})
  , cycleInfo_(*this)
  , goalData_(*this)
  , fieldDimensions_(*this)
  , odometryOffset_(*this)
  , landmarkModel_(*this)
  , goalPostBuffer_()
  , optimalGoalPostDistance_(fieldDimensions_->goalInnerWidth + fieldDimensions_->goalPostDiameter)
{
}

void LandmarkFilter::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  // Add new goal posts to buffer and eventually remove outdated ones
  updateGoalPosts();
  // Combine goal posts to goals
  assembleGoals();
  debug().update(mount_, *this);
  debug().update(mount_ + ".LandmarkModel", *landmarkModel_);
}

void LandmarkFilter::updateGoalPosts()
{
  if (!bufferGoalPosts_())
  {
    // clear buffer if buffering is switched off
    goalPostBuffer_.clear();
  }
  else
  {
    const Pose inverseOdometryOffset = odometryOffset_->odometryOffset.inverse();
    for (auto& goalPost : goalPostBuffer_)
    {
      // apply inverse odometry to the goal posts (in order to move them in relative coordinates)
      goalPost.position = inverseOdometryOffset * goalPost.position;
    }
  }
  // add new goal posts to buffer (only if new data available)
  if (goalData_->timestamp != lastTimestamp_)
  {
    lastTimestamp_ = goalData_->timestamp;
    for (auto& newGoalPostPosition : goalData_->posts)
    {
      // add new data to buffer
      GoalPost newGoalPost = GoalPost(newGoalPostPosition, cycleInfo_->startTime);
      auto goalPostIt = goalPostBuffer_.begin();
      for (; goalPostIt != goalPostBuffer_.end(); goalPostIt++)
      {
        // merge with existing goal post if within association radius and not from the same image
        if ((goalPostIt->position - newGoalPost.position).norm() < goalPostAssociationRadius_() &&
            goalPostIt->timestampLastSeen != newGoalPost.timestampLastSeen)
        {
          *goalPostIt = newGoalPost;
          break;
        }
      }
      if (goalPostIt == goalPostBuffer_.end())
      {
        goalPostBuffer_.push_front(newGoalPost);
      }
    }
  }
  // if buffering is switched on, old goal posts need to be removed from the buffer, if not the buffer is cleared in the beginning of this method anyway
  if (bufferGoalPosts_())
  {
    for (auto goalPostIt = goalPostBuffer_.begin(); goalPostIt != goalPostBuffer_.end();)
    {
      if (cycleInfo_->getTimeDiff(goalPostIt->timestampLastSeen) > maxGoalPostAge_())
      {
        goalPostIt = goalPostBuffer_.erase(goalPostIt);
      }
      else
      {
        goalPostIt++;
      }
    }
  }
}

void LandmarkFilter::assembleGoals()
{
  if (goalPostBuffer_.size() < 2)
  {
    return;
  }

  // Check all unique combinations of two goal posts.
  for (auto post1 = goalPostBuffer_.begin(); std::next(post1) != goalPostBuffer_.end(); post1++)
  {
    for (auto post2 = std::next(post1); post2 != goalPostBuffer_.end(); post2++)
    {
      float dist = (post1->position - post2->position).norm();
      if (std::abs(dist - optimalGoalPostDistance_) < maxGoalPostDistanceDeviation_())
      {
        if (post1->position.y() > post2->position.y())
        {
          landmarkModel_->goals.emplace_back(post1->position, post2->position);
        }
        else
        {
          landmarkModel_->goals.emplace_back(post2->position, post1->position);
        }
      }
    }
  }
}

void LandmarkFilter::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["goalPosts"] << goalPostBuffer_;
}
