// clang-format off
// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// clang-format on
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
// SPDX-FileContributor: Leon Matthes <leon.matthes@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#ifdef CXX_QT_GUI_FEATURE
#include "cxx-qt-lib/qguiapplication.h"

#include <vector>

#include <QtCore/QObject>

namespace {

class ArgsData : public QObject
{
public:
  char** data() { return m_vector.data(); }
  int& size() { return m_size; }
  void push(const std::string& string)
  {
    m_ownedVector.emplace_back(string);
    m_vector.emplace_back(m_ownedVector.back().data());
    m_size = m_ownedVector.size();
  }

private:
  std::vector<std::string> m_ownedVector;
  std::vector<char*> m_vector;
  int m_size = 0;
};

}

namespace rust {
namespace cxxqtlib1 {

::std::int32_t
qguiapplicationExec(QGuiApplication& app)
{
  return static_cast<::std::int32_t>(app.exec());
}

::std::unique_ptr<QGuiApplication>
qguiapplicationNew(::rust::Vec<::rust::String> args)
{
  auto argsData = new ArgsData();
  for (const auto& str : args) {
    // Construct an owned std::string and copy from the rust::String
    argsData->push(std::string(str));
  }

  auto ptr =
    ::std::make_unique<QGuiApplication>(argsData->size(), argsData->data());
  // Set the parent of the ArgsData to QGuiApplication
  // as the vector needs to live as long as the QGuiApplication
  argsData->setParent(ptr.get());

  return ptr;
}

}
}
#endif
