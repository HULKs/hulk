#include "LabelProvider.hpp"
#include "Data/ReplayData.hpp"
#include "Tools/Math/Eigen.hpp"
#include <Tools/Storage/UniValue/UniValue2Json.hpp>
#include <fstream>

LabelProvider::LabelProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , imageData_(*this)
  , labelData_(*this)
{
}

void LabelProvider::cycle()
{
#ifndef REPLAY
  return;
#endif
  ReplayFrame frame;
  if (!robotInterface().getFakeData().getFakeData(frame))
  {
    return;
  }
  const std::string jsonFile = frame.image + ".json";
  Json::Reader reader;
  std::ifstream stream(jsonFile);
  if (!stream.good())
  {
    Log(LogLevel::ERROR) << "Couldn't find frame specific json file. You may need to call the "
                            "replay binary from inside the replay.json folder.";
    return;
  }
  Json::Value tmp;
  reader.parse(stream, tmp);
  Uni::Value uni = Uni::Converter::toUniValue(tmp);
  labelData_->fromValue(uni);
  labelData_->image = frame.image;
  for (auto& labelBox : labelData_->boxes)
  {
    labelBox.box.topLeft.x() = imageData_->image422.size.x() * labelBox.start.x;
    labelBox.box.topLeft.y() = imageData_->image422.size.y() * labelBox.start.y;
    labelBox.box.bottomRight.x() =
        imageData_->image422.size.x() * (labelBox.start.x + labelBox.size.x);
    labelBox.box.bottomRight.y() =
        imageData_->image422.size.y() * (labelBox.start.y + labelBox.size.y);
  }
  for (auto& labelLine : labelData_->lines)
  {
    labelLine.line.p1.x() = static_cast<int>(imageData_->image422.size.x() * labelLine.start.x);
    labelLine.line.p1.y() = static_cast<int>(imageData_->image422.size.y() * labelLine.start.y);
    labelLine.line.p2.x() = static_cast<int>(imageData_->image422.size.x() * labelLine.end.x);
    labelLine.line.p2.y() = static_cast<int>(imageData_->image422.size.y() * labelLine.end.y);
  }
}
