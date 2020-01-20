#pragma once

#include <string>
#include <vector>

#include "Tools/Storage/UniValue/UniValue.h"


/**
 * @class MotionFile represents a Motion, that is stored on hard drive
 * This file format supports
 * - position commands
 * - hardness commands
 *
 * @example MotionFile:
 *
 * @author Finn Poppinga
 */
class MotionFile : public Uni::To, public Uni::From
{
public:
  /**
   * @struct Command represents a command
   * This can either be a position or a hardness command.
   * @author Finn Poppinga
   */
  struct Command : public Uni::To, public Uni::From
  {
    void toValue(Uni::Value& value) const
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["time"] << time;
      value["parameters"] << parameters;
    }
    void fromValue(const Uni::Value& value)
    {
      value["time"] >> time;
      value["parameters"] >> parameters;
    }
    /// the relative time weight of this command.
    /*
     * Every command has a specific time weight. All individual
     * weights are accumulated and every command is executed in a
     * time that is calculated as follows:
     * AbsoluteTime * (TimeWeight/SumOfTimeWeights).
     */
    int time;
    /// the parameters (joint angles or stiffnesses) for this time step
    std::vector<float> parameters;
  };
  /**
   * @struct Header represents the header of a MotionFile
   * @author Finn Poppinga
   */
  struct Header : public Uni::To, public Uni::From
  {
    void toValue(Uni::Value& value) const
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["joints"] << joints;
      value["time"] << time;
      value["title"] << title;
      value["version"] << version;
    }
    void fromValue(const Uni::Value& value)
    {
      value["joints"] >> joints;
      value["time"] >> time;
      value["title"] >> title;
      value["version"] >> version;
    }
    /// the joints and the order of the joints which the MotionFile accesses
    std::vector<int> joints;
    /// the absolute time that the motion should take
    int time;
    /// the title of the MotionFile
    std::string title;
    /// the version of the MotionFile format used
    std::string version;
  };
  /**
   * @brief loadFromFile loads a MotionFile from a given location
   * @param filename the filepath from which the MotionFile is loaded
   * @return whether loading was successful
   */
  bool loadFromFile(const std::string& filename);
  /**
   * @brief saveToFile saves a MotionFile to a given location
   * @param filename the filepath to which the MotionFile is saved
   * @return whether saving was successful
   */
  bool saveToFile(const std::string& filename) const;
  /**
   * @brief verify checks whether the file is valid (all joint arrays are large enough etc.)
   * @return true iff the motion file is valid
   */
  bool verify() const;
  /**
   * @brief toValue converts a MotionFile to a Uni::Value
   * @param value the value that is overwritten with the serialized MotionFile
   */
  void toValue(Uni::Value& value) const;
  /**
   * @brief fromValue converts a Uni::Value to a MotionFile
   * @param value the value from which the MotionFile is deserialized
   */
  void fromValue(const Uni::Value& value);
  /// the header of the motion file
  Header header;
  /// the commands for the joint angles
  std::vector<Command> position;
  /// the commands for the joint stiffnesses
  std::vector<Command> stiffness;
};
