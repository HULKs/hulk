#include "Hardware/Webots/WebotsFakeData.hpp"
#include "Framework/Log/Log.hpp"
#include "Hardware/Webots/WebotsInterface.hpp"
#include <algorithm>
#include <cmath>
#include <webots/Node.hpp>

WebotsFakeData::WebotsFakeData(WebotsInterface& robotInterface)
  : robotInterface_{robotInterface}
{
  const auto* children{robotInterface_.getRoot()->getField("children")};
  for (int index{0}; index < children->getCount(); ++index)
  {
    auto* const node{children->getMFNode(index)};
    if (const auto* const nameField{node->getField("name")}; nameField != nullptr)
    {
      const auto name{nameField->getSFString()};
      if (name == "ball")
      {
        Log<M_TUHHSDK>{LogLevel::INFO} << "Found ball (" << node->getTypeName() << ")";
        ball_ = node;
        continue;
      }
      if (node->getBaseTypeName() == "Robot" && node != robotInterface_.getSelf())
      {
        Log<M_TUHHSDK>{LogLevel::INFO} << "Found robot (" << node->getTypeName() << ")";
        otherRobots_.emplace_back(node);
        continue;
      }
    }
  }
  assert(ball_ != nullptr);
}

void WebotsFakeData::lock()
{
  mutex_.lock();
}

void WebotsFakeData::unlock()
{
  mutex_.unlock();
}

bool WebotsFakeData::readFakeRobotPose(Pose& fakeData)
{
  std::lock_guard lock{mutex_};
  const auto* const position{robotInterface_.getSelf()->getPosition()};
  const auto* const rotationMatrix{robotInterface_.getSelf()->getOrientation()};
  // position and rotationMatrix have swapped y- and z-axis, original z-axis inverted
  fakeData = {static_cast<float>(position[0]), static_cast<float>(-position[2]),
              static_cast<float>(std::atan2(-rotationMatrix[6], rotationMatrix[0]))};
  return true;
}

bool WebotsFakeData::readFakeBallPosition(Vector2f& fakeData)
{
  std::lock_guard lock{mutex_};
  const auto* const position{ball_->getPosition()};
  // position has swapped y- and z-axis, original z-axis inverted
  fakeData = {static_cast<float>(position[0]), static_cast<float>(-position[2])};
  return true;
}

bool WebotsFakeData::readFakeRobotPositions(VecVector2f& fakeData)
{
  std::lock_guard lock{mutex_};
  fakeData.clear();
  std::transform(otherRobots_.begin(), otherRobots_.end(), std::back_inserter(fakeData),
                 [](const webots::Node* const node) -> Vector2f {
                   const auto* const position{node->getPosition()};
                   // position has swapped y- and z-axis, original z-axis inverted
                   return {static_cast<float>(position[0]), static_cast<float>(-position[2])};
                 });
  return true;
}

bool WebotsFakeData::getFakeDataInternal(const std::type_index& /*id*/, DataTypeBase& /*data*/)
{
  return false;
}
