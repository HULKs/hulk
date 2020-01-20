#include <fstream>
#include <iostream>

#include "Definitions/keys.h"
#include "Tools/Storage/UniValue/UniValue2Json.hpp"

#include "MotionFile.hpp"


bool MotionFile::loadFromFile(const std::string& filename)
{
  Json::Reader reader;
  Json::Value root;
  std::ifstream f(filename);
  if (!f.is_open())
  {
    Log(LogLevel::ERROR) << "MotionFile " << filename << " could not be opened!";
    return false;
  }
  try
  {
    reader.parse(f, root);
  }
  catch (...)
  {
    Log(LogLevel::ERROR) << "Could not parse MotionFile " << filename;
    return false;
  }
  fromValue(Uni::Converter::toUniValue(root));
  return true;
}

bool MotionFile::saveToFile(const std::string& filename) const
{
  std::ofstream f(filename);
  if (!f.is_open())
  {
    return false;
  }
  f << Uni::Converter::toJsonString(*this, true);
  return f.good();
}

bool MotionFile::verify() const
{
  if (header.time < 0)
  {
    Log(LogLevel::ERROR) << "MotionFile " << header.title << ": header time is < 0!";
    return false;
  }
  if (header.joints.empty() || header.joints.size() > keys::joints::JOINTS_MAX)
  {
    Log(LogLevel::ERROR) << "MotionFile " << header.title
                         << ": invalid number of joints specified in header!";
    return false;
  }
  for (auto joint : header.joints)
  {
    if (joint < 0 || joint >= keys::joints::JOINTS_MAX)
    {
      Log(LogLevel::ERROR) << "MotionFile " << header.title
                           << ": invalid joint specified in header";
      return false;
    }
  }
  if (header.version != "2.0")
  {
    Log(LogLevel::ERROR) << "MotionFile " << header.title << ": version not 2.0!";
    return false;
  }
  for (auto& pos : position)
  {
    if (pos.time < 0)
    {
      Log(LogLevel::ERROR) << "MotionFile " << header.title << ": position time is < 0!";
      return false;
    }
    if (pos.parameters.size() != header.joints.size())
    {
      Log(LogLevel::ERROR)
          << "MotionFile " << header.title
          << ": number joints in position do not match joints specified in header!";
      return false;
    }
  }
  for (auto& stiff : stiffness)
  {
    if (stiff.time < 0)
    {
      Log(LogLevel::ERROR) << "MotionFile " << header.title << ": stiffness time is < 0!";
      return false;
    }
    if (stiff.parameters.size() != header.joints.size())
    {
      Log(LogLevel::ERROR)
          << "MotionFile " << header.title
          << ": number joints in stiffness do not match joints specified in header!";
      return false;
    }
  }
  return true;
}

void MotionFile::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["header"] << header;
  value["position"] << position;
  value["stiffness"] << stiffness;
}

void MotionFile::fromValue(const Uni::Value& value)
{
  value["header"] >> header;
  value["position"] >> position;
  value["stiffness"] >> stiffness;
}
