// decompress.cpp
//

#undef  UNICODE
#undef  _UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <tchar.h>
#include <strsafe.h>
#include <stdlib.h>

#include <cstdint>

#include <Esent.h>

#define ESENT                                           1
#define JET_cbKeyMost_OLD                               256
#define JET_resoperSize                                 0
#define JET_IOPriorityLowForCheckpoint                  0
#define JET_IOPriorityLowForScavenge                    0
#define JET_efvExtHdrRootFieldAutoIncStorageReleased    1
#define JET_efvLogtimeGenMaxRequired                    5
#define JET_dbstateIncrementalReseedInProgress          -2
#define JET_dbstateRevertInProgress                     -3
#define JET_dbstateDirtyAndPatchedShutdown              -4

#define COLLAssertTrack( X )
#define MSINTERNAL

using JET_RESID                 = int;
using JET_RESOPER               = int;
using JET_RECSIZE3              = void;
using JET_ENGINEFORMATVERSION   = long;
using JET_DBINFOMISC7           = void;
using JET_PFNINITCALLBACK       = void *;
using JET_THREADSTATS4          = void *;

BOOL FOSDllUp();
BOOL FOSLayerUp();

#include "ms/cc.hxx"
#include "ms/types.hxx"
#include "ms/memory.hxx"
#include "ms/osstd.hxx"
#include "ms/math.hxx"
#include "ms/thread.hxx"
#include "ms/sync.hxx"
#include "ms/syncu.hxx"
#include "ms/task.hxx"
#include "ms/cprintf.hxx"
#include "ms/trace.hxx"
#include "ms/oseventtrace.hxx"
#include "ms/sysinfo.hxx"
#include "ms/norm.hxx"
#include "ms/privconsts.h"
#include "ms/cresmgr.hxx"
#include "ms/string.hxx"
#include "ms/_jet.hxx"
#include "ms/stat.hxx"
#include "ms/daedef.hxx"

#include "ms/_xpress/xpress.h"

struct CDataCompressor {
    enum COMPRESSION_SCHEME
    {
        COMPRESS_NONE,
        COMPRESS_7BITASCII = 0x1,
        COMPRESS_7BITUNICODE = 0x2,
        COMPRESS_XPRESS = 0x3,
        COMPRESS_SCRUB = 0x4,
        COMPRESS_XPRESS9 = 0x5,
        COMPRESS_XPRESS10 = 0x6,
        COMPRESS_MAXIMUM = 0x1f,
    };

    CDataCompressor() {
        FOSSyncPreinit();
        ErrInit();
    }
    ERR ErrInit(/* const INT cbMin, const INT cbMax */)
    {
        ERR err = JET_errSuccess;

        //Assert( cbMin >= 0 );
        //m_cbMin = cbMin;
        //Assert( cbMax >= cbMin );
        //m_cbMax = cbMax;

        //m_cencodeCachedMax = OSSyncGetProcessorCountMax();
        m_cdecodeCachedMax = OSSyncGetProcessorCountMax();

        //Assert( m_rgencodeXpress == NULL );
        //Assert( m_rgdecodeXpress == NULL );
        //Alloc( m_rgencodeXpress = new XpressEncodeStream[ m_cencodeCachedMax ]() );
        Alloc( m_rgdecodeXpress = new XpressDecodeStream[ m_cdecodeCachedMax ]() );

        #ifdef XPRESS9_COMPRESSION
        Assert( m_rgencodeXpress9 == NULL );
        Assert( m_rgdecodeXpress9 == NULL );
        Alloc( m_rgencodeXpress9 = new XPRESS9_ENCODER[ m_cencodeCachedMax ]() );
        Alloc( m_rgdecodeXpress9 = new XPRESS9_DECODER[ m_cdecodeCachedMax ]() );
        #endif

        return err;

    HandleError:
        //delete[] m_rgencodeXpress;
        //m_rgencodeXpress = NULL;
        delete[] m_rgdecodeXpress;
        m_rgdecodeXpress = NULL;
        #ifdef XPRESS9_COMPRESSION
        delete[] m_rgencodeXpress9;
        m_rgencodeXpress9 = NULL;
        delete[] m_rgdecodeXpress9;
        m_rgdecodeXpress9 = NULL;
        #endif

        return err;
    }


    ERR ErrDecompress(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual );
    ERR ErrDecompress7BitAscii_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual );
    ERR ErrDecompress7BitUnicode_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual );
    ERR ErrDecompressXpress_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual);
    ERR ErrDecompressScrub_(
        const DATA& data );
    #ifdef XPRESS9_COMPRESSION
    ERR ErrDecompressXpress9_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual,
        IDataCompressorStats * const pstats );
    #endif
    #ifdef XPRESS10_COMPRESSION
    ERR ErrDecompressXpress10_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual,
        IDataCompressorStats * const pstats,
        const BOOL fForceSoftwareDecompression,
        BOOL * pfUsedCorsica );
    #endif

    INT m_cdecodeCachedMax;
    XpressDecodeStream* m_rgdecodeXpress;
    ERR ErrXpressDecodeOpen_( _Out_ XpressDecodeStream * const pdecode );
    void XpressDecodeClose_( XpressDecodeStream decode );
    void XpressDecodeRelease_( XpressDecodeStream decode );

    static void * XPRESS_CALL PvXpressAlloc_(
        _In_opt_ void * pvContext,
        INT             cbAlloc );
    static void XPRESS_CALL XpressFree_(
        _In_opt_ void *             pvContext,
        _Post_ptr_invalid_ void *   pvAlloc );
};

extern "C" {
    uint32_t decompress(uint8_t const *data,
                        uint32_t data_size,
                        uint8_t * const out_buffer,
                        uint32_t out_buffer_size,
                        uint32_t * decompressed) {
        static CDataCompressor  worker;
        DATA    dataCompressed;

        dataCompressed.SetPv((void *)data);
        dataCompressed.SetCb(data_size);

        INT     actual = 0;
        ERR     res = worker.ErrDecompress(dataCompressed, out_buffer, out_buffer_size, &actual);
        *decompressed = actual;

        return res;
    }
}

ERR CDataCompressor::ErrDecompress(
    const DATA& dataCompressed,
    _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
    const INT cbDataMax,
    _Out_ INT * const pcbDataActual )
{
    ERR err = JET_errSuccess;
    BOOL fUnused = fFalse;

    const BYTE bHeader = *(BYTE *)dataCompressed.Pv();
    const BYTE bIdentifier = bHeader >> 3;

    switch( bIdentifier )
    {
        case COMPRESS_7BITASCII:
            Call( ErrDecompress7BitAscii_( dataCompressed, pbData, cbDataMax, pcbDataActual ) );
            break;
        case COMPRESS_7BITUNICODE:
            Call( ErrDecompress7BitUnicode_( dataCompressed, pbData, cbDataMax, pcbDataActual ) );
            break;
        case COMPRESS_XPRESS:
            Call( ErrDecompressXpress_( dataCompressed, pbData, cbDataMax, pcbDataActual) );
            break;
        case COMPRESS_SCRUB:
            Expected( pbData == NULL );
            Expected( cbDataMax == 0 );
            Assert( g_fRepair || FNegTest( fInvalidUsage ) );
            *pcbDataActual = 0;
            Call( ErrDecompressScrub_( dataCompressed ) );
            break;
            #ifdef XPRESS9_COMPRESSION
        case COMPRESS_XPRESS9:
            Call( ErrDecompressXpress9_( dataCompressed, pbData, cbDataMax, pcbDataActual, pstats ) );
            break;
            #endif
            #ifdef XPRESS10_COMPRESSION
        case COMPRESS_XPRESS10:
            Call( ErrDecompressXpress10_( dataCompressed, pbData, cbDataMax, pcbDataActual, pstats, fFalse, &fUnused ) );
            break;
            #endif
        default:
            *pcbDataActual = 0;
            Call( ErrERRCheck( JET_errDecompressionFailed ) );
            break;
    }

    if( *pcbDataActual > cbDataMax )
    {
        Assert( JET_wrnBufferTruncated == err );
    }

HandleError:
    Expected( ( bIdentifier != COMPRESS_SCRUB ) || ( err == wrnRECCompressionScrubDetected ) );

    return err;
}

ERR CDataCompressor::ErrDecompress7BitAscii_(
    const DATA& dataCompressed,
    _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
    const INT cbDataMax,
    _Out_ INT * const pcbDataActual )
{
    ERR err = JET_errSuccess;

    const BYTE * const pbCompressed = (BYTE *)dataCompressed.Pv();
    const BYTE bHeader = *(BYTE *)dataCompressed.Pv();
    const BYTE bIdentifier = bHeader >> 3;
    Assert( bIdentifier == COMPRESS_7BITASCII );
    const INT cbitFinal = (bHeader & 0x7)+1;
    Assert( cbitFinal > 0 );
    Assert( cbitFinal <= 8 );

    const INT cbitTotal = ( ( dataCompressed.Cb() - 2 ) * 8 ) + cbitFinal;
    Assert( 0 == cbitTotal % 7 );
    const INT cbTotal = cbitTotal / 7;

    *pcbDataActual = cbTotal;
    if( cbTotal > cbDataMax )
    {
        err = ErrERRCheck( JET_wrnBufferTruncated );
    }

    if( 0 == cbDataMax || NULL == pbData )
    {
        goto HandleError;
    }

    {
        const INT ibDataMax = min(cbTotal, cbDataMax);

        INT ibCompressed = 1;
        INT ibitCompressed = 0;
        for (INT ibData = 0; ibData < ibDataMax; ++ibData)     {
            Assert(ibCompressed < dataCompressed.Cb());
            BYTE bDecompressed;
            if (ibitCompressed <= 1)         {
                const BYTE bCompressed = pbCompressed[ibCompressed];
                bDecompressed = (BYTE)((bCompressed >> ibitCompressed) & 0x7F);
            }
            else         {
                Assert(ibCompressed < dataCompressed.Cb() - 1);
                const WORD wCompressed = (WORD)pbCompressed[ibCompressed] | ((WORD)pbCompressed[ibCompressed + 1] << 8);
                bDecompressed = (BYTE)((wCompressed >> ibitCompressed) & 0x7F);
            }
            pbData[ibData] = bDecompressed;
            ibitCompressed += 7;
            if (ibitCompressed >= 8)         {
                ibitCompressed = (ibitCompressed % 8);
                ++ibCompressed;
            }
        }
    }
HandleError:
    #pragma prefast(suppress : 26030, "In case of JET_wrnBufferTruncated, we return what the buffer size should be.")
    return err;
}

ERR CDataCompressor::ErrDecompress7BitUnicode_(
    const DATA& dataCompressed,
    _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
    const INT cbDataMax,
    _Out_ INT * const pcbDataActual )
{
    ERR err = JET_errSuccess;

    const BYTE * const pbCompressed = (BYTE *)dataCompressed.Pv();
    const BYTE bHeader = *(BYTE *)dataCompressed.Pv();
    const BYTE bIdentifier = bHeader >> 3;
    Assert( bIdentifier == COMPRESS_7BITUNICODE );
    const INT cbitFinal = (bHeader & 0x7)+1;
    Assert( cbitFinal > 0 );
    Assert( cbitFinal <= 8 );

    const INT cbitTotal = ( ( dataCompressed.Cb() - 2 ) * 8 ) + cbitFinal;
    Assert( 0 == cbitTotal % 7 );
    const INT cwTotal = cbitTotal / 7;

    *pcbDataActual = cwTotal * sizeof(WORD);
    if( *pcbDataActual > cbDataMax )
    {
        err = ErrERRCheck( JET_wrnBufferTruncated );
    }

    if( 0 == cbDataMax || NULL == pbData )
    {
        goto HandleError;
    }

    {
        const INT ibDataMax = min(*pcbDataActual, cbDataMax);

        INT ibCompressed = 1;
        INT ibitCompressed = 0;
        for (INT ibData = 0; ibData < ibDataMax; )     {
            Assert(ibCompressed < dataCompressed.Cb());
            BYTE bDecompressed;
            if (ibitCompressed <= 1)         {
                const BYTE bCompressed = pbCompressed[ibCompressed];
                bDecompressed = (BYTE)((bCompressed >> ibitCompressed) & 0x7F);
            }
            else         {
                Assert(ibCompressed < dataCompressed.Cb() - 1);
                const WORD wCompressed = (WORD)pbCompressed[ibCompressed] | ((WORD)pbCompressed[ibCompressed + 1] << 8);
                bDecompressed = (BYTE)((wCompressed >> ibitCompressed) & 0x7F);
            }

            pbData[ibData++] = bDecompressed;
            if (ibData >= ibDataMax)         {
                break;
            }
            pbData[ibData++] = 0x0;
            if (ibData >= ibDataMax)         {
                break;
            }

            ibitCompressed += 7;
            if (ibitCompressed >= 8)         {
                ibitCompressed = (ibitCompressed % 8);
                ++ibCompressed;
            }
        }
    }

HandleError:
    #pragma prefast(suppress : 26030, "In case of JET_wrnBufferTruncated, we return what the buffer size should be.")
    return err;
}

ERR CDataCompressor::ErrDecompressXpress_(
    const DATA& dataCompressed,
    _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
    const INT cbDataMax,
    _Out_ INT * const pcbDataActual)
{
    ERR err = JET_errSuccess;
    PERFOptDeclare( const HRT hrtStart = HrtHRTCount() );

    XpressDecodeStream decode = 0;

    const BYTE * const  pbCompressed    = (BYTE *)dataCompressed.Pv();
    const UnalignedLittleEndian<WORD> * const pwSize    = (UnalignedLittleEndian<WORD> *)(pbCompressed + 1);
    const INT cbUncompressed            = *pwSize;

    const BYTE bHeader      = *(BYTE *)dataCompressed.Pv();
    OnDebug( const BYTE bIdentifier     = bHeader >> 3 );
    Assert( bIdentifier == COMPRESS_XPRESS );

    *pcbDataActual = cbUncompressed;

    const INT cbHeader              = sizeof(BYTE) + sizeof(WORD);
    const BYTE * pbCompressedData   = (BYTE *)dataCompressed.Pv() + cbHeader;
    const INT cbCompressedData      = dataCompressed.Cb() - cbHeader;
    const INT cbWanted              = min( *pcbDataActual, cbDataMax );

    if ( NULL == pbData || 0 == cbDataMax )
    {
        err = ErrERRCheck( JET_wrnBufferTruncated );
        goto HandleError;
    }

    Assert( cbWanted <= cbUncompressed );

    Call( ErrXpressDecodeOpen_( &decode ) );

    {
        const INT cbDecoded = XpressDecode(
            decode,
            pbData,
            cbUncompressed,
            cbWanted,
            pbCompressedData,
            cbCompressedData);
        if (-1 == cbDecoded)     {
            Call(ErrERRCheck(JET_errDecompressionFailed));
        }
        Assert(cbDecoded == cbWanted);

        if (cbUncompressed > cbDataMax)     {
            err = ErrERRCheck(JET_wrnBufferTruncated);
        }
    }
HandleError:
    XpressDecodeClose_( decode );

    if ( err >= JET_errSuccess )
    {
        PERFOpt( pstats->AddDecompressionBytes( cbWanted ) );
        PERFOpt( pstats->IncDecompressionCalls() );
        PERFOpt( pstats->AddDecompressionDhrts( HrtHRTCount() - hrtStart ) );
    }

    #pragma prefast(suppress : 26030, "In case of JET_wrnBufferTruncated, we return what the buffer size should be.")
    return err;
}

ERR CDataCompressor::ErrDecompressScrub_(
    const DATA& data )
{
    #ifdef DEBUG
    const BYTE * const pbData = (BYTE *)data.Pv();
    const INT cbData = data.Cb();
    Assert( cbData >= 1 );
    const BYTE bHeader = pbData[0];
    const BYTE bIdentifier = bHeader >> 3;

    Assert( bIdentifier == COMPRESS_SCRUB );
    Expected( ( bHeader & 0x07 ) == 0 );

    if ( cbData > 1 )
    {
        const CHAR chKnownPattern = pbData[1];

        Expected( chKnownPattern == chSCRUBLegacyLVChunkFill || chKnownPattern == chSCRUBDBMaintLVChunkFill );

        const INT cbKnownPattern = cbData - 1;
        BYTE* pbKnownPattern = new BYTE[cbKnownPattern];
        if ( pbKnownPattern != NULL )
        {
            memset( pbKnownPattern, chKnownPattern, cbKnownPattern );
            Assert( memcmp( pbKnownPattern, pbData + 1, cbKnownPattern ) == 0 );
        }
        delete [] pbKnownPattern;
    }
    #endif
    return 0; //ErrERRCheck( wrnRECCompressionScrubDetected );
}

ERR CDataCompressor::ErrXpressDecodeOpen_( _Out_ XpressDecodeStream * const pdecode )
{
    C_ASSERT( sizeof(void*) == sizeof(XpressDecodeStream) );

    *pdecode = 0;

    XpressDecodeStream decode = GetCachedPtr<XpressDecodeStream>( m_rgdecodeXpress, m_cdecodeCachedMax );
    if( 0 != decode )
    {
        *pdecode = decode;
        return JET_errSuccess;
    }

    decode = XpressDecodeCreate(
        0,
        PvXpressAlloc_ );
    if (NULL == decode)
    {
        return ErrERRCheck( JET_errOutOfMemory );
    }
    *pdecode = decode;
    return JET_errSuccess;
}

void CDataCompressor::XpressDecodeClose_( XpressDecodeStream decode )
{
    if ( decode )
    {
        if( FCachePtr<XpressDecodeStream>( decode, m_rgdecodeXpress, m_cdecodeCachedMax ) )
        {
            return;
        }

        XpressDecodeRelease_( decode );
    }
}

void * XPRESS_CALL CDataCompressor::PvXpressAlloc_( _In_opt_ void * pvContext, INT cbAlloc )
{
    return new BYTE[cbAlloc];
}

void XPRESS_CALL CDataCompressor::XpressFree_( _In_opt_ void * pvContext, _Post_ptr_invalid_ void * pvAlloc )
{
    delete [] pvAlloc;
}


void CDataCompressor::XpressDecodeRelease_( XpressDecodeStream decode )
{
    if ( decode )
    {
        XpressDecodeClose( decode, 0, XpressFree_ );
    }
}

#ifndef RUST_LIBRARY
//======================================================
#include <iostream>
#include <vector>
#include <string>

#define errRECCannotCompress                -418  /* column cannot be compressed */

static const INT xpressLegacyCompressionLevel = 2;

INT m_cbMax = 100;  //???
XpressEncodeStream* m_rgencodeXpress;
INT m_cencodeCachedMax;

void * XPRESS_CALL /*CDataCompressor::*/PvXpressAlloc_( _In_opt_ void * pvContext, INT cbAlloc )
{
    return new BYTE[cbAlloc];
}

ERR /*CDataCompressor::*/ErrXpressEncodeOpen_( _Out_ XpressEncodeStream * const pencode )
{
    C_ASSERT( sizeof(void*) == sizeof(XpressEncodeStream) );
    *pencode = 0;

    m_cencodeCachedMax = OSSyncGetProcessorCountMax();
    m_rgencodeXpress =  new XpressEncodeStream[ m_cencodeCachedMax ]();

    XpressEncodeStream encode = GetCachedPtr<XpressEncodeStream>( m_rgencodeXpress, m_cencodeCachedMax );
    if ( 0 != encode )
    {
        *pencode = encode;
        return JET_errSuccess;
    }

    encode = XpressEncodeCreate(
        m_cbMax,
        0,
        PvXpressAlloc_,
        xpressLegacyCompressionLevel );
    if( NULL == encode )
    {
        return ErrERRCheck( JET_errOutOfMemory );
    }
    *pencode = encode;
    return JET_errSuccess;
}

void XPRESS_CALL /*CDataCompressor::*/XpressFree_( _In_opt_ void * pvContext, _Post_ptr_invalid_ void * pvAlloc )
{
    delete [] pvAlloc;
}

void /*CDataCompressor::*/XpressEncodeRelease_( XpressEncodeStream encode )
{
    if ( encode )
    {
        XpressEncodeClose( encode, 0, XpressFree_ );
    }
}


void /*CDataCompressor::*/XpressEncodeClose_( XpressEncodeStream encode )
{
    if ( encode )
    {
        if( FCachePtr<XpressEncodeStream>( encode, m_rgencodeXpress, m_cencodeCachedMax ) )
        {
            return;
        }

        XpressEncodeRelease_( encode );
    }
}

ERR /*CDataCompressor::*/ErrCompressXpress_(
    const DATA& data,
    _Out_writes_bytes_to_( cbDataCompressedMax, *pcbDataCompressedActual ) BYTE * const pbDataCompressed,
    const INT cbDataCompressedMax,
    _Out_ INT * const pcbDataCompressedActual)
{
    PERFOptDeclare( const HRT hrtStart = HrtHRTCount() );

    Assert( data.Cb() >= m_cbMin );
    Assert( data.Cb() <= wMax );
    Assert( data.Cb() <= m_cbMax );
    Assert( pstats );

    ERR err = JET_errSuccess;

    XpressEncodeStream encode = 0;
    Call( ErrXpressEncodeOpen_( &encode ) );

    {
        const INT cbReserved = sizeof(BYTE) + sizeof(WORD);

        const INT cbCompressed = XpressEncode(
            encode,
            pbDataCompressed + cbReserved,
            cbDataCompressedMax - cbReserved,
            data.Pv(),
            data.Cb(),
            0,
            0,
            0 );
        Assert( cbCompressed <= data.Cb() );

        if( cbCompressed == 0 || cbCompressed + cbReserved >= data.Cb() )
        {
            err = ErrERRCheck( errRECCannotCompress );
        }
        else
        {
            BYTE * const pbSignature    = pbDataCompressed;
            UnalignedLittleEndian<WORD> * const pwSize  = (UnalignedLittleEndian<WORD> *)(pbSignature+1);

            *pbSignature    = ( CDataCompressor::COMPRESS_XPRESS << 3 );
            *pwSize         = (WORD)data.Cb();
            *pcbDataCompressedActual = cbCompressed + cbReserved;
        }
    }

HandleError:
    XpressEncodeClose_( encode );

    if ( err == JET_errSuccess )
    {
        PERFOpt( pstats->AddUncompressedBytes( data.Cb() ) );
        PERFOpt( pstats->AddCompressedBytes( *pcbDataCompressedActual ) );
        PERFOpt( pstats->IncCompressionCalls() );
        PERFOpt( pstats->AddCompressionDhrts( HrtHRTCount() - hrtStart ) );
    }

    return err;
}

int main() {
    using Buffer = std::vector<uint8_t>;
    DATA            data;
    INT             res;
    std::string     src {"===========================test string data to check compression/decompression=================="};
    Buffer          compressed_data(src.size() + 8);
    INT             compressed = 0;
    uint32_t        decompressed = 0;
    CDataCompressor to_call_init;

    data.SetPv((void *)src.data());
    data.SetCb(src.size());

    if (JET_errSuccess != (res = ErrCompressXpress_(data, compressed_data.data(), compressed_data.size(), &compressed))) {
        std::cerr << "ERROR: compress " << res;
        return 1;
    }

    if (JET_wrnBufferTruncated == (res = decompress(compressed_data.data(), compressed, nullptr, 0, &decompressed))) {
        Buffer           decompressed_data(decompressed);

        if (JET_errSuccess == (res = decompress(compressed_data.data(), compressed, 
                                                decompressed_data.data(), decompressed_data.size(), &decompressed))) {
            std::string  str((char*)decompressed_data.data(), decompressed_data.size());

            std::cout << "decompressed = " << decompressed << ": '" << str << "'\n";
        }
        else {
            std::cerr << "ERROR: decompress " << res;
            return 1;
        }
    }
    else {
        std::cerr << "ERROR: decompressed (" << res << ") != JET_wrnBufferTruncated!";
        return 1;
    }

    return 0;
}
#endif //RUST_LIBRARY
