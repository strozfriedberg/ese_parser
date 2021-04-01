// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.


namespace Microsoft
{
#if (ESENT)
namespace Windows
#else
namespace Exchange
#endif
{
namespace Isam
{


/*MSINTERNAL*/ enum class MJET_ASSERT
{
    Exit = 0x0000,
    Break = 0x0001,
    MsgBox = 0x0002,
    Stop = 0x0004,
    SkippableMsgBox = 0x0008,
    SkipAll = 0x0010,
    Crash = 0x0020,
    FailFast = 0x0040,
};

}
}
}

#define JET_AssertFailFast          (UINT)Microsoft::Windows::Isam::MJET_ASSERT::FailFast
#define JET_AssertExit              (UINT)Microsoft::Windows::Isam::MJET_ASSERT::Exit
#define JET_AssertBreak             (UINT)Microsoft::Windows::Isam::MJET_ASSERT::Break
#define JET_AssertMsgBox            (UINT)Microsoft::Windows::Isam::MJET_ASSERT::MsgBox
#define JET_AssertSkippableMsgBox   (UINT)Microsoft::Windows::Isam::MJET_ASSERT::SkippableMsgBox
#define JET_AssertStop              (UINT)Microsoft::Windows::Isam::MJET_ASSERT::Stop
#define JET_AssertCrash             (UINT)Microsoft::Windows::Isam::MJET_ASSERT::Crash
#define JET_AssertSkipAll           (UINT)Microsoft::Windows::Isam::MJET_ASSERT::SkipAll