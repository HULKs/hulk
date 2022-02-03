#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


class KeeperAction : public DataType<KeeperAction>
{
public:
  /// the name of this DataType
  DataTypeName name__{"KeeperAction"};

  /**
   * @enum Type enumerates the possible types of action for a keeper
   * all values must be powers of two for the permission management to work
   */
  enum class Type
  {
    // block as muy of the own goal as possible
    BLOCK_GOAL = 1,
    // perform squat motion
    SQUAT = 2,
  };

  // sum all actions the keeper is allowed to perform
  unsigned int permission = static_cast<unsigned int>(Type::BLOCK_GOAL);

  struct Action : public Uni::To, public Uni::From
  {
    /// the type of the action
    Type type;
    /// the position walk to
    Pose pose = Pose();
    /// true iff this struct is valid
    bool valid = false;

    Action()
      : type(Type::BLOCK_GOAL)
      , pose(Pose())
      , valid(false)
    {
    }

    Action(Type t)
      : type(t)
      , pose(Pose())
      , valid(true)
    {
    }

    Action(Type t, Pose p)
      : type(t)
      , pose(p)
      , valid(true)
    {
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["type"] << static_cast<int>(type);
      value["pose"] << pose;
      value["valid"] << valid;
    }

    void fromValue(const Uni::Value& value) override
    {
      int valueRead;
      value["type"] >> valueRead;
      type = static_cast<Type>(valueRead);
      value["pose"] >> pose;
      value["valid"] >> valid;
    }
  };

  /// vector of all keeper actions produced by the KeeperActionProvider module
  std::vector<Action> actions;

  /// the action to be performed by the keeper
  Action action;

  /// indicate if Keeper wants to play ball
  bool wantsToPlayBall = false;

  void reset() override
  {
    actions.clear();
    action = Action();
    wantsToPlayBall = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["permission"] << permission;
    value["actions"] << actions;
    value["action"] << action;
    value["wantsToPlayBall"] << wantsToPlayBall;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["permission"] >> permission;
    value["actions"] >> actions;
    value["action"] >> action;
    value["wantsToPlayBall"] >> wantsToPlayBall;
  }
};
