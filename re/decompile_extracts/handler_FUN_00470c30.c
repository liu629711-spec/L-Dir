// ==== FUN_00470c30 @ 00470c30 ====

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined8
FUN_00470c30(longlong param_1,ulonglong param_2,LPCWSTR param_3,LPCWSTR param_4,undefined4 *param_5)

{
  undefined8 *puVar1;
  HRESULT HVar2;
  int iVar3;
  int iVar4;
  uint uVar5;
  LSTATUS LVar6;
  longlong lVar7;
  longlong *plVar8;
  undefined8 *puVar9;
  wchar_t *pwVar10;
  undefined8 uVar11;
  ulonglong *puVar12;
  HWND pHVar13;
  HMODULE pHVar14;
  HKEY hKey;
  wchar_t *pwVar15;
  LPCWSTR pWVar16;
  ulonglong uVar17;
  LPCWSTR pWVar18;
  int *piVar19;
  bool bVar20;
  int local_848;
  undefined4 local_844;
  wchar_t *local_840;
  undefined *local_838;
  undefined8 *local_830;
  undefined *local_828;
  undefined8 *local_820;
  longlong *local_818;
  LPCWSTR local_810;
  LPCWSTR local_808;
  longlong *local_800;
  undefined8 *local_7f8;
  LPCWSTR local_7f0;
  undefined8 *local_7e8;
  undefined *local_7e0;
  undefined8 *local_7d8;
  SHELLFLAGSTATE local_7d0 [2];
  wchar_t *local_7c8;
  undefined8 *local_7c0;
  longlong local_7b8;
  longlong local_7b0;
  ulonglong local_7a8;
  undefined8 *local_7a0;
  LPCWSTR local_798;
  undefined8 *local_790;
  ulonglong local_788;
  undefined8 *local_780;
  longlong local_778;
  HKEY local_770;
  longlong local_768;
  undefined8 *local_760;
  undefined8 *local_758;
  ulonglong local_750;
  longlong local_748;
  ulonglong local_740;
  LPCWSTR local_738;
  LPCWSTR local_730;
  undefined8 local_728;
  ushort *local_720;
  LPCWSTR local_718;
  undefined **local_710;
  undefined8 local_708;
  undefined8 local_700;
  longlong local_6f8;
  SHELLSTATEA local_6f0;
  undefined **local_6c8;
  undefined8 local_6c0;
  void *local_6a0;
  undefined8 local_698;
  undefined *local_688;
  undefined8 local_680;
  longlong local_678;
  longlong local_670;
  longlong local_668;
  longlong local_660;
  longlong local_658;
  longlong local_650;
  longlong local_648;
  longlong local_640;
  longlong local_638;
  longlong local_630;
  longlong local_628;
  longlong local_620;
  longlong local_618;
  longlong local_610;
  longlong local_608;
  longlong local_600;
  longlong local_5f8;
  longlong local_5f0;
  longlong local_5e8;
  longlong local_5e0;
  longlong local_5d8;
  longlong local_5d0;
  longlong local_5c8;
  longlong local_5c0;
  longlong local_5b8;
  longlong local_5b0;
  longlong local_5a8;
  longlong local_5a0;
  longlong local_598;
  longlong local_590;
  longlong local_588;
  longlong local_580;
  longlong local_578;
  longlong local_570;
  longlong local_568;
  longlong local_560;
  longlong local_558;
  longlong local_550;
  longlong local_548;
  longlong local_540;
  longlong local_538;
  longlong local_530;
  longlong local_528;
  longlong local_520;
  longlong local_518;
  longlong local_510;
  longlong local_508;
  longlong local_500;
  longlong local_4f8;
  longlong local_4f0;
  longlong local_4e8;
  longlong local_4e0;
  longlong local_4d8;
  longlong local_4d0;
  longlong local_4c8;
  longlong local_4c0;
  longlong local_4b8;
  longlong local_4b0;
  longlong local_4a8;
  longlong local_4a0;
  longlong local_498;
  longlong local_490;
  longlong local_488;
  longlong local_480;
  longlong local_478;
  longlong local_470;
  longlong local_468;
  longlong local_460;
  longlong local_458;
  longlong local_450;
  longlong local_448;
  longlong local_440;
  longlong local_438;
  longlong local_430;
  longlong local_428;
  longlong local_420;
  longlong local_418;
  longlong local_410;
  longlong local_408;
  ulonglong local_400;
  longlong local_3f8;
  ulonglong local_3f0;
  longlong local_3e8;
  ulonglong local_3e0;
  longlong local_3d8;
  ulonglong local_3d0;
  longlong local_3c8;
  longlong local_3c0;
  longlong local_3b8;
  longlong local_3b0;
  longlong local_3a8;
  longlong local_3a0;
  longlong local_398;
  undefined8 local_390 [2];
  longlong local_380;
  undefined8 local_378 [2];
  longlong local_368;
  longlong local_360;
  longlong local_358;
  longlong local_350;
  longlong local_348;
  longlong local_340;
  undefined8 local_338 [26];
  WCHAR local_268 [264];
  undefined8 local_58;
  
  local_58 = DAT_00547f48;
  pwVar15 = param_3;
  pWVar16 = param_4;
  local_810 = param_4;
  if ((DAT_00551be0 & 1) == 0) {
    DAT_00551be0 = DAT_00551be0 | 1;
    DAT_00551bd8 = PTR_DAT_00546000;
    FUN_0050ee00(FUN_005239fc);
  }
  *param_5 = 0;
  local_848 = 0;
  iVar4 = (int)param_3;
  if (param_3 + -0x59cd < (LPCWSTR)0x10) {
    FUN_0045a798(param_1,iVar4);
    local_848 = 2;
  }
  else {
    if (param_3 == (LPCWSTR)0xbd7b) {
      DAT_00546af8 = (uint)(DAT_00546af8 == 0);
      FUN_004f6cec(L"disable_dark_theme",DAT_00546af8);
      puVar12 = FUN_00461840(&local_740);
      local_828 = PTR_DAT_00546000;
      iVar4 = lstrlenW(L"\\start2.qdr");
      FUN_00402204((longlong *)&local_828,*(int *)((undefined8 *)*puVar12 + -1),
                   (undefined8 *)*puVar12,iVar4,(undefined8 *)L"\\start2.qdr");
      piVar19 = (int *)(local_740 - 0xc);
      if (piVar19 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar4 = *piVar19;
        *piVar19 = *piVar19 + -1;
        UNLOCK();
        if (iVar4 + -1 < 1) {
          thunk_FUN_0050e3a0((LPVOID)(local_740 - 0xc));
        }
      }
      plVar8 = FUN_0040329c(&local_380,(longlong *)&local_828);
      FUN_0045cf90(param_1,plVar8,1,1,0,0,0);
      plVar8 = FUN_0040329c(&local_3c0,(longlong *)&local_828);
      FUN_0045a1e0(param_1,plVar8);
      PostQuitMessage(0xaff);
      piVar19 = (int *)(local_828 + -0xc);
      if (piVar19 == (int *)PTR_DAT_00546008) {
        return 0;
      }
      LOCK();
      iVar4 = *piVar19;
      *piVar19 = *piVar19 + -1;
      UNLOCK();
      if (0 < iVar4 + -1) {
        return 0;
      }
      thunk_FUN_0050e3a0(piVar19);
      return 0;
    }
    if (param_3 == (LPCWSTR)0xb408) {
      puVar12 = (ulonglong *)&DAT_005511f8;
      pwVar15 = (wchar_t *)(ulonglong)(DAT_005511f0 == 0);
      pWVar16 = (LPCWSTR)0x0;
      DAT_005511f0 = (uint)(DAT_005511f0 == 0);
      FUN_0047d8e4(DAT_00554108,(LPCWSTR)&DAT_005511f8,pwVar15,(LPCWSTR)0x0);
      goto LAB_004720c7;
    }
    if (param_3 == (LPCWSTR)0x85b1) {
      lVar7 = FUN_00456560(param_1);
      if (lVar7 == 0) {
        return 0;
      }
      lVar7 = FUN_00456560(param_1);
      FUN_004c3294(lVar7);
    }
  }
  if (param_3 == (LPCWSTR)0x85b9) {
    lVar7 = FUN_00456560(param_1);
    if (lVar7 != 0) {
      lVar7 = FUN_00456560(param_1);
      FUN_004c0a84(lVar7);
    }
LAB_00473cc4:
    if (local_848 == 0) {
      return 0;
    }
  }
  else {
    if (param_3 == (LPCWSTR)0xb380) {
      *param_5 = 1;
      FUN_004f8990(*(HWND *)(param_1 + 0x4de0),*(longlong *)(param_1 + 0x4de8));
    }
    if (param_3 == (LPCWSTR)0xb37f) {
      *param_5 = 1;
      if (param_4 != (LPCWSTR)0x3e8) goto LAB_00473cc4;
      FUN_004562b8(param_1,1);
      plVar8 = FUN_00407e50(&local_3b0,L"");
      pWVar16 = (LPCWSTR)0x1;
      pwVar15 = (LPCWSTR)0x0;
      FUN_0045ad08(param_1,plVar8,0,1);
      FUN_0045e064(param_1);
    }
    if (param_3 == (LPCWSTR)0xb3f5) {
      lVar7 = FUN_00456560(param_1);
      if (lVar7 != 0) {
        FUN_004c0ce4(lVar7);
      }
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb3f6) {
      lVar7 = FUN_00456560(param_1);
      if (lVar7 != 0) {
        FUN_004bf848(lVar7);
        FUN_004cf390(lVar7);
      }
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0x8012) {
      *param_5 = 1;
      FUN_0045a6f0(param_1);
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0x8503) {
      DAT_0055c530 = -(ulonglong)(param_1 != 0) & param_1 + 0x60U;
      FUN_00503a2c(*(HWND *)(param_1 + 8));
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb325) {
      lVar7 = FUN_00456560(param_1);
      if (lVar7 == 0) goto LAB_00473cc4;
      puVar9 = FUN_004c6fbc(lVar7,local_390);
      plVar8 = FUN_004ef70c((longlong)puVar9,&local_3a0);
      FUN_004e2734(plVar8);
      puVar9 = local_390;
LAB_00472dc5:
      FUN_004efeb4(puVar9);
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb335) {
      lVar7 = FUN_00456560(param_1);
      if (lVar7 != 0) {
        pwVar15 = (LPCWSTR)0x0;
LAB_00471031:
        FUN_004cc524(lVar7,0,(int)pwVar15);
      }
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xbd78) {
      lVar7 = FUN_00456560(param_1);
      if (lVar7 != 0) {
        pwVar15 = (LPCWSTR)0x1;
        goto LAB_00471031;
      }
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb409) {
      FUN_0045766c(param_1);
    }
    if (param_3 == (LPCWSTR)0xb360) {
      FUN_00415420(0,0);
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb358) {
      pWVar16 = (LPCWSTR)&DAT_00534378;
      pwVar15 = (LPCWSTR)0x15;
      local_818 = (longlong *)0x0;
      HVar2 = CoCreateInstance((IID *)&DAT_00534368,(LPUNKNOWN)0x0,0x15,(IID *)&DAT_00534378,
                               &local_818);
      plVar8 = local_818;
      if (-1 < HVar2) {
        (**(code **)(*local_818 + 0x80))();
        plVar8 = local_818;
      }
LAB_00471108:
      if (plVar8 != (longlong *)0x0) {
        (**(code **)(*plVar8 + 0x10))();
      }
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb359) {
      pWVar16 = (LPCWSTR)&DAT_00534378;
      pwVar15 = (LPCWSTR)0x15;
      local_800 = (longlong *)0x0;
      HVar2 = CoCreateInstance((IID *)&DAT_00534368,(LPUNKNOWN)0x0,0x15,(IID *)&DAT_00534378,
                               &local_800);
      plVar8 = local_800;
      if (-1 < HVar2) {
        (**(code **)(*local_800 + 0xa0))();
        plVar8 = local_800;
      }
      goto LAB_00471108;
    }
    if (param_3 == (LPCWSTR)0xb336) {
      bVar20 = *(int *)(param_1 + 0xf0) == 0;
      pwVar15 = (wchar_t *)(ulonglong)bVar20;
      *(uint *)(param_1 + 0xf0) = (uint)bVar20;
      plVar8 = FUN_00407e50(&local_360,L"adress_sw_driver");
      FUN_004554a0(param_1,plVar8,pwVar15);
      FUN_00420d10(param_1 + 0xa8);
      goto LAB_00473cc4;
    }
    if ((param_3 < (LPCWSTR)0xb327) || ((LPCWSTR)0xb329 < param_3)) {
      if (param_3 == (LPCWSTR)0xb339) {
        DAT_00550ea4 = (uint)(DAT_00550ea4 == 0);
        plVar8 = FUN_00407e50(&local_668,L"refresh_keep_sel");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00550ea4;
      }
      else {
        if (param_3 != (LPCWSTR)0xb33a) {
          if (param_3 == (LPCWSTR)0xe124) {
            lVar7 = FUN_00456560(param_1);
            if (lVar7 != 0) {
              FUN_004c027c(lVar7);
            }
            goto LAB_00473cc4;
          }
          if (param_3 == (LPCWSTR)0x8552) {
            DAT_00555294 = (uint)(DAT_00555294 == 0);
            plVar8 = FUN_00407e50(&local_658,L"only_label_or_icon_clk");
            pwVar15 = (wchar_t *)(ulonglong)DAT_00555294;
            goto LAB_00473971;
          }
          if (param_3 == (LPCWSTR)0x8553) {
            DAT_00555298 = (uint)(DAT_00555298 == 0);
            plVar8 = FUN_00407e50(&local_4c8,L"no_keep_grouping");
            pwVar15 = (wchar_t *)(ulonglong)DAT_00555298;
            goto LAB_00473971;
          }
          if (param_3 != (LPCWSTR)0x851b) {
            if (((LPCWSTR)0xab7b < param_3) && (param_3 < (LPCWSTR)0xaba9)) {
              (*(code *)**(undefined8 **)(param_1 + 0x60))
                        ((undefined8 *)(param_1 + 0x60),iVar4 + -0xab7c);
              plVar8 = FUN_00407e50(&local_648,L"m_lang_id");
              FUN_004554a0(param_1,plVar8,(ulonglong)DAT_00546c60);
            }
            goto LAB_004715bf;
          }
          FUN_004a1d10((ulonglong *)&local_840,0xf4241,pwVar15,(ulonglong)pWVar16);
          FUN_0040331c((longlong *)&local_7f0,"");
          if (((*(int *)(local_840 + -4) < 0) ||
              (pwVar10 = wcsstr(local_840,L"http://"), pwVar10 == (wchar_t *)0x0)) ||
             ((int)((longlong)pwVar10 - (longlong)local_840 >> 1) != 0)) {
            pwVar10 = local_840;
            local_7e0 = PTR_DAT_00546000;
            iVar4 = lstrlenW(L"http://");
            pWVar16 = (LPCWSTR)(ulonglong)*(uint *)(pwVar10 + -4);
            pwVar15 = L"http://";
            FUN_00402204((longlong *)&local_7e0,iVar4,(undefined8 *)L"http://",
                         *(uint *)(pwVar10 + -4),(undefined8 *)local_840);
            FUN_00402154((longlong *)&local_7f0,(longlong *)&local_7e0);
            piVar19 = (int *)(local_7e0 + -0xc);
            if (piVar19 != (int *)PTR_DAT_00546008) {
              LOCK();
              iVar4 = *piVar19;
              *piVar19 = *piVar19 + -1;
              UNLOCK();
              if (iVar4 + -1 < 1) {
                thunk_FUN_0050e3a0(piVar19);
              }
            }
          }
          else {
            FUN_00402154((longlong *)&local_7f0,(longlong *)&local_840);
          }
          if (((*(int *)(local_840 + -4) < 0) ||
              (pwVar10 = wcsstr(local_840,L"."), pwVar10 == (wchar_t *)0x0)) ||
             ((int)((longlong)pwVar10 - (longlong)local_840 >> 1) < 0)) {
            FUN_0040331c((longlong *)&local_7c0,"\nNo homepage!!!");
            puVar12 = FUN_004a1d10(&local_7a8,0xf4242,pwVar15,(ulonglong)pWVar16);
            local_738 = (LPCWSTR)PTR_DAT_00546000;
            FUN_00402204((longlong *)&local_738,*(int *)((undefined8 *)*puVar12 + -1),
                         (undefined8 *)*puVar12,*(int *)(local_7c0 + -1),local_7c0);
            pWVar16 = local_738;
            MessageBoxW(*(HWND *)(param_1 + 8),local_738,L"",0);
            pWVar16 = pWVar16 + -6;
            if (pWVar16 != (LPCWSTR)PTR_DAT_00546008) {
              LOCK();
              iVar4 = *(int *)pWVar16;
              *(int *)pWVar16 = *(int *)pWVar16 + -1;
              UNLOCK();
              if (iVar4 + -1 < 1) {
                thunk_FUN_0050e3a0(pWVar16);
              }
            }
            piVar19 = (int *)(local_7a8 - 0xc);
            if (piVar19 != (int *)PTR_DAT_00546008) {
              LOCK();
              iVar4 = *piVar19;
              *piVar19 = *piVar19 + -1;
              UNLOCK();
              if (iVar4 + -1 < 1) {
                thunk_FUN_0050e3a0((LPVOID)(local_7a8 - 0xc));
              }
            }
            piVar19 = (int *)((longlong)local_7c0 + -0xc);
            if (piVar19 != (int *)PTR_DAT_00546008) {
              LOCK();
              iVar4 = *piVar19;
              *piVar19 = *piVar19 + -1;
              UNLOCK();
              if (iVar4 + -1 < 1) {
                thunk_FUN_0050e3a0(piVar19);
              }
            }
          }
          pWVar16 = (LPCWSTR)0x0;
          pwVar15 = local_7f0;
          ShellExecuteW(*(HWND *)(param_1 + 8),L"open",local_7f0,(LPCWSTR)0x0,(LPCWSTR)0x0,5);
          local_7f0 = local_7f0 + -6;
          if (local_7f0 != (LPCWSTR)PTR_DAT_00546008) {
            LOCK();
            iVar4 = *(int *)local_7f0;
            *(int *)local_7f0 = *(int *)local_7f0 + -1;
            UNLOCK();
            if (iVar4 + -1 < 1) {
              thunk_FUN_0050e3a0(local_7f0);
            }
          }
          pwVar10 = local_840 + -6;
          if (pwVar10 == (wchar_t *)PTR_DAT_00546008) goto LAB_00473cc4;
          LOCK();
          iVar4 = *(int *)pwVar10;
          *(int *)pwVar10 = *(int *)pwVar10 + -1;
          UNLOCK();
          if (0 < iVar4 + -1) goto LAB_00473cc4;
LAB_00473394:
          local_840 = local_840 + -6;
LAB_00473cbf:
          thunk_FUN_0050e3a0(local_840);
          goto LAB_00473cc4;
        }
        DAT_00550eac = (uint)(DAT_00550eac == 0);
        plVar8 = FUN_00407e50(&local_408,L"refresh_keep_scroll");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00550eac;
      }
LAB_00473971:
      FUN_004554a0(param_1,plVar8,pwVar15);
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb327) {
      DAT_00555288 = 0;
    }
    if (param_3 == (LPCWSTR)0xb328) {
      DAT_00555288 = 1;
    }
    if (param_3 == (LPCWSTR)0xb329) {
      DAT_00555288 = 2;
    }
    plVar8 = FUN_00407e50(&local_350,L"show_ext_in_type");
    FUN_004554a0(param_1,plVar8,(ulonglong)DAT_00555288);
    local_848 = 1;
LAB_004715bf:
    pwVar15 = (LPCWSTR)0xb24f;
    if (param_3 == (LPCWSTR)0xb24f) {
      uVar11 = FUN_0045a344(param_1,param_2 & 0xffffffff,0xb24f,(ulonglong)param_4,param_5);
      return uVar11;
    }
    if (param_3 != (LPCWSTR)0x84d0) {
      if (param_3 == (LPCWSTR)0xb337) {
        DAT_00555a68 = (uint)(DAT_00555a68 == 0);
        plVar8 = FUN_00407e50(&local_638,L"tree_no_expando");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00555a68;
        goto LAB_00471620;
      }
      if (param_3 == (LPCWSTR)0xb357) {
        DAT_00550f6c = (uint)(DAT_00550f6c == 0);
        plVar8 = FUN_00407e50(&local_4b8,L"tree_auto_expand_from_list");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00550f6c;
        FUN_004554a0(param_1,plVar8,pwVar15);
      }
      if (param_3 == (LPCWSTR)0xb31e) {
        DAT_00546c68 = (uint)(DAT_00546c68 == 0);
        plVar8 = FUN_00407e50(&local_628,L"tree_auto_sel");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00546c68;
        goto LAB_00473971;
      }
      if (param_3 == (LPCWSTR)0x84de) {
        DAT_00547eb0 = 0;
        plVar8 = FUN_00407e50(&local_3f8,L"tree_all_browsable");
        pwVar15 = (LPCWSTR)0x0;
        goto LAB_00471620;
      }
      if (param_3 == (LPCWSTR)0x84df) {
        DAT_00547eb0 = 1;
        plVar8 = FUN_00407e50(&local_618,L"tree_all_browsable");
        pwVar15 = (LPCWSTR)0x1;
        goto LAB_00471620;
      }
      if (param_3 == (LPCWSTR)0x84e6) {
        *(undefined4 *)(param_1 + 0x204) = 0;
        plVar8 = FUN_00407e50(&local_4a8,L"adresbar_style");
        uVar11 = 0;
LAB_00471797:
        FUN_004554a0(param_1,plVar8,uVar11);
        FUN_0041f8ec(param_1 + 0xa8,*(int *)(param_1 + 0x204));
        pWVar16 = (LPCWSTR)0x105;
        pwVar15 = (LPCWSTR)0x0;
        RedrawWindow(*(HWND *)(param_1 + 0xb0),(RECT *)0x0,(HRGN)0x0,0x105);
        goto LAB_00473cc4;
      }
      if (param_3 == (LPCWSTR)0x84e7) {
        *(undefined4 *)(param_1 + 0x204) = 1;
        plVar8 = FUN_00407e50(&local_608,L"adresbar_style");
        uVar11 = 1;
        goto LAB_00471797;
      }
      if (param_3 == (LPCWSTR)0x84e9) {
        *(undefined4 *)(param_1 + 0x208) = 0;
        plVar8 = FUN_00407e50(&local_398,L"adresbar_styleB");
        pwVar15 = (LPCWSTR)0x0;
LAB_0047182e:
        FUN_004554a0(param_1,plVar8,pwVar15);
        FUN_00420dd0(*(int *)(param_1 + 0x208),param_1 + 0xa8);
        goto LAB_00473cc4;
      }
      if (param_3 == (LPCWSTR)0x84ea) {
        *(undefined4 *)(param_1 + 0x208) = 1;
        plVar8 = FUN_00407e50(&local_5f8,L"adresbar_styleB");
        pwVar15 = (LPCWSTR)0x1;
        goto LAB_0047182e;
      }
      if (param_3 == (LPCWSTR)0x84e8) {
        ShowWindow(*(HWND *)(param_1 + 0x368),0);
        DAT_005463f4 = (uint)(DAT_005463f4 == 0);
        plVar8 = FUN_00407e50(&local_498,L"adresbar_use_highlighted");
        uVar5 = DAT_005463f4;
LAB_004718e5:
        pwVar15 = (wchar_t *)(ulonglong)uVar5;
        FUN_004554a0(param_1,plVar8,pwVar15);
        ShowWindow(*(HWND *)(param_1 + 0x368),5);
        goto LAB_00473cc4;
      }
      if (param_3 == (LPCWSTR)0x8555) {
        ShowWindow(*(HWND *)(param_1 + 0x368),0);
        DAT_00547bc4 = (uint)(DAT_00547bc4 == 0);
        plVar8 = FUN_00407e50(&local_5e8,L"status_use_highlighted");
        uVar5 = DAT_00547bc4;
        goto LAB_004718e5;
      }
      if (param_3 == (LPCWSTR)0x858b) {
        DAT_00555a7c = (uint)(DAT_00555a7c == 0);
        plVar8 = FUN_00407e50(&local_3e8,L"tree_line_at_root");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00555a7c;
        goto LAB_00471620;
      }
      if (param_3 == (LPCWSTR)0x84d4) {
        DAT_00555a78 = (uint)(DAT_00555a78 == 0);
        plVar8 = FUN_00407e50(&local_5d8,L"tree_disable_line");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00555a78;
        goto LAB_00471620;
      }
      if (param_3 == (LPCWSTR)0xb250) {
        DAT_00555a80 = (uint)(DAT_00555a80 == 0);
        if ((DAT_00555a78 == 0) && (DAT_00555a80 != 0)) {
          FUN_0040331c((longlong *)&local_760," = ");
          puVar12 = FUN_004a3f04(&local_750,0x16b4f,pwVar15,(ulonglong)pWVar16);
          local_830 = (undefined8 *)PTR_DAT_00546000;
          puVar9 = (undefined8 *)*puVar12;
          uVar17 = (ulonglong)*(uint *)(local_760 + -1);
          FUN_00402204((longlong *)&local_830,*(int *)(puVar9 + -1),puVar9,*(uint *)(local_760 + -1)
                       ,local_760);
          puVar12 = FUN_004a3f04(&local_788,0x16b52,puVar9,uVar17);
          puVar9 = local_830;
          local_798 = (LPCWSTR)PTR_DAT_00546000;
          FUN_00402204((longlong *)&local_798,*(int *)(local_830 + -1),local_830,
                       *(int *)((undefined8 *)*puVar12 + -1),(undefined8 *)*puVar12);
          pWVar18 = local_798;
          pWVar16 = (LPCWSTR)0x0;
          MessageBoxW(*(HWND *)(param_1 + 8),local_798,L"",0);
          pWVar18 = pWVar18 + -6;
          if (pWVar18 != (LPCWSTR)PTR_DAT_00546008) {
            LOCK();
            iVar3 = *(int *)pWVar18;
            *(int *)pWVar18 = *(int *)pWVar18 + -1;
            UNLOCK();
            puVar9 = local_830;
            if (iVar3 + -1 < 1) {
              thunk_FUN_0050e3a0(pWVar18);
              puVar9 = local_830;
            }
          }
          piVar19 = (int *)((longlong)puVar9 + -0xc);
          if (piVar19 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar3 = *piVar19;
            *piVar19 = *piVar19 + -1;
            UNLOCK();
            if (iVar3 + -1 < 1) {
              thunk_FUN_0050e3a0(piVar19);
            }
          }
          piVar19 = (int *)(local_750 - 0xc);
          if (piVar19 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar3 = *piVar19;
            *piVar19 = *piVar19 + -1;
            UNLOCK();
            if (iVar3 + -1 < 1) {
              thunk_FUN_0050e3a0((LPVOID)(local_750 - 0xc));
            }
          }
          piVar19 = (int *)(local_788 - 0xc);
          if (piVar19 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar3 = *piVar19;
            *piVar19 = *piVar19 + -1;
            UNLOCK();
            if (iVar3 + -1 < 1) {
              thunk_FUN_0050e3a0((LPVOID)(local_788 - 0xc));
            }
          }
          piVar19 = (int *)((longlong)local_760 + -0xc);
          param_4 = local_810;
          if (piVar19 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar3 = *piVar19;
            *piVar19 = *piVar19 + -1;
            UNLOCK();
            if (iVar3 + -1 < 1) {
              thunk_FUN_0050e3a0(piVar19);
              param_4 = local_810;
            }
          }
        }
        plVar8 = FUN_00407e50(&local_488,L"tree_fullrowselect");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00555a80;
        goto LAB_00471620;
      }
      if (param_3 == (LPCWSTR)0xb251) {
        DAT_00555a84 = (uint)(DAT_00555a84 == 0);
        plVar8 = FUN_00407e50(&local_5c8,L"tree_expand_when_select");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00555a84;
        goto LAB_00473971;
      }
      if (param_3 == (LPCWSTR)0xb252) {
        DAT_00555a88 = (uint)(DAT_00555a88 == 0);
        plVar8 = FUN_00407e50(&local_358,L"tree_single_expand");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00555a88;
        goto LAB_00471620;
      }
      if (param_3 == (LPCWSTR)0xb35c) {
        DAT_00550ed0 = (uint)(DAT_00550ed0 == 0);
        plVar8 = FUN_00407e50(&local_5b8,L"Disable_Win7_Tree");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00550ed0;
        FUN_004554a0(param_1,plVar8,pwVar15);
        FUN_00455d20(param_1,plVar8,pwVar15,(ulonglong)pWVar16);
        goto LAB_00473cc4;
      }
      if ((param_3 < (LPCWSTR)0x852e) || ((LPCWSTR)0x853f < param_3)) {
        if (param_3 != (LPCWSTR)0xb36e) {
          if (param_3 == (LPCWSTR)0xb36f) {
            DAT_00550e70 = 1;
            plVar8 = FUN_00407e50(&local_5a8,L"autocomplete");
            pwVar15 = (LPCWSTR)0x1;
            goto LAB_004726ff;
          }
          if (param_3 == (LPCWSTR)0xb370) {
            DAT_00546c38 = (uint)(DAT_00546c38 == 0);
            plVar8 = FUN_00407e50(&local_3d8,L"autocomplete_url");
            pwVar15 = (wchar_t *)(ulonglong)DAT_00546c38;
            goto LAB_004726ff;
          }
          if (param_3 == (LPCWSTR)0xb375) {
            DAT_00546c40 = 1;
            plVar8 = FUN_00407e50(&local_598,L"address_ico");
            pwVar15 = (LPCWSTR)0x1;
          }
          else if (param_3 == (LPCWSTR)0xb376) {
            DAT_00546c40 = 0;
            plVar8 = FUN_00407e50(&local_468,L"address_ico");
            pwVar15 = (LPCWSTR)0x0;
          }
          else {
            if (param_3 != (LPCWSTR)0xb377) {
              if (param_3 == (LPCWSTR)0x8562) {
                DAT_00550ee4 = 0;
                pwVar15 = L"media_preview_by";
                plVar8 = &local_678;
                goto LAB_00473969;
              }
              if (param_3 == (LPCWSTR)0x8563) {
                DAT_00550ee4 = 1;
                pwVar15 = L"media_preview_by";
                plVar8 = &local_578;
                goto LAB_0047252e;
              }
              if (param_3 != (LPCWSTR)0x8557) {
                if (param_3 == (LPCWSTR)0x8558) {
                  DAT_00546c58 = 2;
                  plVar8 = FUN_00407e50(&local_568,L"show_tabs");
                  pwVar15 = (LPCWSTR)0x2;
                  FUN_004554a0(param_1,plVar8,2);
                  thunk_FUN_00423d10();
                }
                goto LAB_00471e59;
              }
              DAT_00546c58 = 1;
              plVar8 = FUN_00407e50(&local_458,L"show_tabs");
              pwVar15 = (LPCWSTR)0x1;
              goto LAB_004725ad;
            }
            DAT_00546c40 = 2;
            plVar8 = FUN_00407e50(&local_588,L"address_ico");
            pwVar15 = (LPCWSTR)0x2;
          }
          goto LAB_00472a5f;
        }
        DAT_00550e70 = 0;
        plVar8 = FUN_00407e50(&local_478,L"autocomplete");
        pwVar15 = (LPCWSTR)0x0;
        goto LAB_004726ff;
      }
      puVar9 = (undefined8 *)(param_1 + 0x368);
      uVar11 = FUN_004c032c(iVar4,*(HWND *)(-(ulonglong)(param_1 != -0x360) & (ulonglong)puVar9));
      if ((int)uVar11 != 0) {
        ShowWindow((HWND)*puVar9,0);
        ShowWindow((HWND)*puVar9,5);
      }
LAB_00471e59:
      if (param_3 == (LPCWSTR)0xb36c) goto LAB_0047239f;
      if (param_3 == (LPCWSTR)0x8559) {
        DAT_00546c58 = 0;
        plVar8 = FUN_00407e50(&local_3c8,L"show_tabs");
        pwVar15 = (LPCWSTR)0x0;
        goto LAB_004725ad;
      }
      if (param_3 == (LPCWSTR)0xb31d) {
        DAT_00550ed4 = (uint)(DAT_00550ed4 == 0);
        plVar8 = FUN_00407e50(&local_558,L"close_ask");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00550ed4;
        goto LAB_00473971;
      }
      if (param_3 == (LPCWSTR)0x8568) {
        DAT_00547b94 = 0;
        pwVar15 = L"copy_cut_paste_mode";
        plVar8 = &local_448;
        goto LAB_00473969;
      }
      if (param_3 != (LPCWSTR)0x8569) {
        if (param_3 == (LPCWSTR)0x1d0) {
          DAT_00547b98 = (uint)(DAT_00547b98 == 0);
          plVar8 = FUN_00407e50(&local_340,L"m_delete_thread");
          FUN_004554a0(param_1,plVar8,(ulonglong)DAT_00547b98);
          return 1;
        }
        if (param_3 == (LPCWSTR)0x856d) {
          DAT_00547b9c = (uint)(DAT_00547b9c == 0);
          plVar8 = FUN_00407e50(&local_538,L"disable_column_hl_new");
          uVar5 = DAT_00547b9c;
          goto LAB_00472973;
        }
        if (param_3 != (LPCWSTR)0xb256) {
          if (param_3 == (LPCWSTR)0xb257) {
            DAT_00547ba4 = (uint)(DAT_00547ba4 == 0);
            plVar8 = FUN_00407e50(&local_528,L"header_in_all_views");
            pwVar15 = (wchar_t *)(ulonglong)DAT_00547ba4;
            goto LAB_00472a5f;
          }
          if (param_3 == (LPCWSTR)0x85ad) {
            uVar11 = FUN_0043a104();
            if ((int)uVar11 == 0) {
              FUN_0043a514();
            }
            else {
              FUN_0043a680();
            }
            goto LAB_00473cc4;
          }
          if (param_3 == (LPCWSTR)0x85ae) {
            FUN_0043a3f4();
            goto LAB_00473cc4;
          }
          if (param_3 == (LPCWSTR)0xb330) {
            SHGetSettings(local_7d0,0x42023);
            if (SPULL(local_7d0[0],0xb,1) == 0) {
              local_6f0._0_4_ = local_6f0._0_4_ | 0x800;
            }
            else {
              local_6f0._0_4_ = local_6f0._0_4_ & 0xfffff7ff;
            }
            pwVar15 = (LPCWSTR)0x1;
            SHGetSetSettings(&local_6f0,0x2000,1);
            goto LAB_00473cc4;
          }
          if (param_3 == (LPCWSTR)0x8554) {
            DAT_00555268 = (uint)(DAT_00555268 == 0);
            plVar8 = FUN_00407e50(&local_3b8,L"disable_up_down_edit");
            pwVar15 = (wchar_t *)(ulonglong)DAT_00555268;
            goto LAB_00473971;
          }
          if (param_3 == (LPCWSTR)0xb3f1) {
            DAT_0055526c = (uint)(DAT_0055526c == 0);
            plVar8 = FUN_00407e50(&local_518,L"m_disable_grouping");
            pwVar15 = (wchar_t *)(ulonglong)DAT_0055526c;
            goto LAB_004726ff;
          }
          if (param_3 == (LPCWSTR)0x8576) {
            DAT_00546c54 = (uint)(DAT_00546c54 == 0);
            plVar8 = FUN_00407e50(&local_428,L"tab_use_context_menu");
            pwVar15 = (wchar_t *)(ulonglong)DAT_00546c54;
            goto LAB_00473971;
          }
          if ((param_3 < (LPCWSTR)0x8570) || ((LPCWSTR)0x8575 < param_3)) {
            if (param_3 == (LPCWSTR)0x8587) {
              DAT_00550e88 = (uint)(DAT_00550e88 == 0);
              plVar8 = FUN_00407e50(&local_508,L"disable_print_bg_color");
              pwVar15 = (wchar_t *)(ulonglong)DAT_00550e88;
              goto LAB_00473971;
            }
            if (param_3 == (LPCWSTR)0x8589) {
              DAT_00550e8c = (uint)(DAT_00550e8c == 0);
              plVar8 = FUN_00407e50(&local_368,L"disable_print_bg_color");
              pwVar15 = (wchar_t *)(ulonglong)DAT_00550e8c;
              goto LAB_00473971;
            }
            if (param_3 == (LPCWSTR)0x8588) {
              DAT_00550e90 = (uint)(DAT_00550e90 == 0);
              plVar8 = FUN_00407e50(&local_4f8,L"disable_print_txt_color");
              pwVar15 = (wchar_t *)(ulonglong)DAT_00550e90;
              goto LAB_00473971;
            }
            if ((LPCWSTR)0x13 < param_3 + -300000) goto LAB_0047239f;
            FUN_004563a4(iVar4);
            goto LAB_00472354;
          }
          pwVar15 = (LPCWSTR)0x0;
          FUN_0045aa50(param_1,iVar4 - 0x8570,0);
          goto LAB_0047239f;
        }
        DAT_00547ba0 = (uint)(DAT_00547ba0 == 0);
        plVar8 = FUN_00407e50(&local_438,L"noenumrefresh2");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00547ba0;
        FUN_004554a0(param_1,plVar8,pwVar15);
        local_848 = 2;
        lVar7 = FUN_00456560(param_1);
        if (lVar7 != 0) {
          FUN_004c0adc(lVar7);
        }
        goto LAB_00471ff7;
      }
      DAT_00547b94 = 1;
      pwVar15 = L"copy_cut_paste_mode";
      plVar8 = &local_548;
LAB_0047252e:
      plVar8 = FUN_00407e50(plVar8,pwVar15);
      pwVar15 = (LPCWSTR)0x1;
      goto LAB_00473971;
    }
    DAT_00547ea8 = (uint)(DAT_00547ea8 == 0);
    plVar8 = FUN_00407e50(&local_348,L"tree_auto_chk_subdir");
    pwVar15 = (wchar_t *)(ulonglong)DAT_00547ea8;
LAB_00471620:
    FUN_004554a0(param_1,plVar8,pwVar15);
LAB_00472354:
    if (*(longlong *)(param_1 + 0xa20) != 0) {
      FUN_004f51c8(param_1 + 0xa18);
    }
    FUN_004f51c8(param_1 + 0x52a8);
    FUN_004f51c8(param_1 + 24000);
    FUN_004f51c8(param_1 + 0x68d8);
    FUN_004f51c8(param_1 + 0x73f0);
LAB_0047239f:
    if ((((((param_3 != (LPCWSTR)0xe123) && (param_3 != (LPCWSTR)0xe125)) &&
          (param_3 != (LPCWSTR)0xe122)) &&
         ((param_3 != (LPCWSTR)0xe12a && (param_3 != (LPCWSTR)0xb37b)))) &&
        (param_3 != (LPCWSTR)0x8556)) &&
       (((param_3 != (LPCWSTR)0x8565 && (param_3 != (LPCWSTR)0x8566)) &&
        ((param_3 != (LPCWSTR)0xb25a &&
         (((param_3 != (LPCWSTR)0xb258 && (param_3 != (LPCWSTR)0xb25b)) &&
          (param_3 != (LPCWSTR)0x8567)))))))) {
      iVar3 = 0xe12b;
      if ((param_3 == (LPCWSTR)0xe12b) || (iVar3 = 0xe12c, param_3 == (LPCWSTR)0xe12c)) {
        FUN_00467958(param_1,iVar3);
        goto LAB_00473cc4;
      }
      if (param_3 == (LPCWSTR)0xb323) {
        DAT_00546628 = (uint)(DAT_00546628 == 0);
        plVar8 = FUN_00407e50(&local_418,L"tab_close_by_lbt_dblclk");
        pwVar15 = (wchar_t *)(ulonglong)DAT_00546628;
      }
      else if (param_3 == (LPCWSTR)0xb324) {
        DAT_0054662c = (uint)(DAT_0054662c == 0);
        plVar8 = FUN_00407e50(&local_4e8,L"tab_close_by_mbt_clk");
        pwVar15 = (wchar_t *)(ulonglong)DAT_0054662c;
      }
      else {
        if (param_3 != (LPCWSTR)0xb32c) goto LAB_0047250c;
        DAT_0054b518 = 0;
        pwVar15 = L"tab_show_tooltips";
        plVar8 = &local_3a8;
LAB_00473969:
        plVar8 = FUN_00407e50(plVar8,pwVar15);
        pwVar15 = (LPCWSTR)0x0;
      }
      goto LAB_00473971;
    }
    lVar7 = FUN_00456560(param_1);
    if (lVar7 != 0) {
      FUN_004d1e5c(lVar7,iVar4);
    }
LAB_0047250c:
    if (param_3 == (LPCWSTR)0xb32d) {
      DAT_0054b518 = 1;
      pwVar15 = L"tab_show_tooltips";
      plVar8 = &local_4d8;
      goto LAB_0047252e;
    }
    if (param_3 == (LPCWSTR)0xb32e) {
      DAT_00546630 = (uint)(DAT_00546630 == 0);
      plVar8 = FUN_00407e50(&local_670,L"tab_new_tab_l_clk");
      pwVar15 = (wchar_t *)(ulonglong)DAT_00546630;
      goto LAB_00473971;
    }
    if (param_3 == (LPCWSTR)0xbb81) {
      DAT_0054b51c = (uint)(DAT_0054b51c == 0);
      plVar8 = FUN_00407e50(&local_660,L"multiline_tab");
      pwVar15 = (wchar_t *)(ulonglong)DAT_0054b51c;
LAB_004725ad:
      FUN_004554a0(param_1,plVar8,pwVar15);
      thunk_FUN_00423d10();
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb32f) {
      DAT_0054b520 = (uint)(DAT_0054b520 == 0);
      plVar8 = FUN_00407e50(&local_650,L"tab_new_tab_l_dblclk");
      pwVar15 = (wchar_t *)(ulonglong)DAT_0054b520;
      goto LAB_00473971;
    }
    if (param_3 == (LPCWSTR)0xb332) {
      DAT_00546c48 = (uint)(DAT_00546c48 == 0);
      plVar8 = FUN_00407e50(&local_640,L"new_tab_m_clk_tree");
      pwVar15 = (wchar_t *)(ulonglong)DAT_00546c48;
      goto LAB_00473971;
    }
    if (param_3 == (LPCWSTR)0xb331) {
      DAT_00546c44 = (uint)(DAT_00546c44 == 0);
      plVar8 = FUN_00407e50(&local_630,L"new_tab_m_clk_list");
      pwVar15 = (wchar_t *)(ulonglong)DAT_00546c44;
      FUN_004554a0(param_1,plVar8,pwVar15);
    }
    if (param_3 == (LPCWSTR)0xbd86) {
      DAT_00546c3c = (uint)(DAT_00546c3c == 0);
      plVar8 = FUN_00407e50(&local_620,L"m_tab_active_line");
      FUN_004554a0(param_1,plVar8,(ulonglong)DAT_00546c3c);
      pwVar15 = (LPCWSTR)0x0;
      EnumChildWindows(*(HWND *)(param_1 + 8),FUN_00455da8,0);
    }
    if (param_3 == (LPCWSTR)0xbd87) {
      DAT_0054b514 = (uint)(DAT_0054b514 == 0);
      plVar8 = FUN_00407e50(&local_610,L"m_draw_close_button");
      pwVar15 = (wchar_t *)(ulonglong)DAT_0054b514;
LAB_004726ff:
      FUN_004554a0(param_1,plVar8,pwVar15);
      local_848 = 3;
      goto LAB_00471ff7;
    }
    if (param_3 == (LPCWSTR)0xb35f) {
      lVar7 = FUN_00456560(param_1);
      if (lVar7 != 0) {
        FUN_004efec8(&local_728);
        FUN_004cb13c(lVar7,(longlong)&local_728);
        FUN_004becac(local_378);
        if (DAT_00549bd8 == (HKEY)0x0) {
          SHGetMalloc((IMalloc **)&DAT_00549bd8);
        }
        else {
          (**(code **)(*(longlong *)DAT_00549bd8 + 8))();
        }
        iVar4 = FUN_004eaf7c(local_720);
        puVar12 = FUN_005102e0((longlong)iVar4 + 2,1);
        FUN_0050daf0(puVar12,0,(longlong)(int)(iVar4 + 2U));
        if (puVar12 != (ulonglong *)0x0) {
          FUN_0050dc70(puVar12,(undefined8 *)local_720,(longlong)iVar4);
        }
        FUN_0049f4a8((longlong *)&local_808,(byte *)puVar12,iVar4 + 2U);
        pWVar16 = (LPCWSTR)0x0;
        pwVar15 = local_808;
        FUN_0047d63c(L"Start",L"default_tab",local_808,(LPCWSTR)0x0);
        pWVar18 = local_808 + -6;
        if (pWVar18 != (LPCWSTR)PTR_DAT_00546008) {
          LOCK();
          iVar4 = *(int *)pWVar18;
          *(int *)pWVar18 = *(int *)pWVar18 + -1;
          UNLOCK();
          if (iVar4 + -1 < 1) {
            thunk_FUN_0050e3a0(local_808 + -6);
          }
        }
        if ((DAT_00549bd8 != (HKEY)0x0) &&
           (iVar4 = (**(code **)(*(longlong *)DAT_00549bd8 + 0x10))(), iVar4 == 0)) {
          DAT_00549bd8 = (HKEY)0x0;
        }
        FUN_004bea74(local_378);
        puVar9 = &local_728;
        goto LAB_00472dc5;
      }
      goto LAB_00473cc4;
    }
    if (param_3 == (LPCWSTR)0xb25c) {
      DAT_00546634 = 0;
      pwVar15 = L"tab_width_mode";
      plVar8 = &local_600;
      goto LAB_00473969;
    }
    if (param_3 == (LPCWSTR)0xb25e) {
      DAT_00546634 = 2;
      DAT_00546638 = FUN_004225d8(DAT_00546638,2);
      plVar8 = FUN_00407e50(&local_5f0,L"tab_width_mode");
      FUN_004554a0(param_1,plVar8,(ulonglong)DAT_00546634);
      plVar8 = FUN_00407e50(&local_5e0,L"tab_max_width");
      pwVar15 = (wchar_t *)(ulonglong)DAT_00546638;
      goto LAB_00473971;
    }
    if (param_3 == (LPCWSTR)0xb333) {
      DAT_00550ebc = (uint)(DAT_00550ebc == 0);
      plVar8 = FUN_00407e50(&local_5d0,L"list_view_auto_chk_sel");
      pwVar15 = (wchar_t *)(ulonglong)DAT_00550ebc;
LAB_00472a5f:
      FUN_004554a0(param_1,plVar8,pwVar15);
      local_848 = 2;
    }
    else {
      if (param_3 != (LPCWSTR)0xb35a) {
        if (param_3 == (LPCWSTR)0xb35d) {
          FUN_00460504(param_1);
        }
        else {
          if (param_3 != (LPCWSTR)0xb35e) {
            if (param_3 == (LPCWSTR)0xb362) {
              DAT_00550eb0 = (uint)(DAT_00550eb0 == 0);
              plVar8 = FUN_00407e50(&local_5b0,L"use_space_bar_for_list_scroll");
              pwVar15 = (wchar_t *)(ulonglong)DAT_00550eb0;
            }
            else if (param_3 == (LPCWSTR)0xb361) {
              DAT_00550eb4 = (uint)(DAT_00550eb4 == 0);
              plVar8 = FUN_00407e50(&local_5a0,L"expand_favorites_at_start");
              pwVar15 = (wchar_t *)(ulonglong)DAT_00550eb4;
            }
            else {
              if (param_3 == (LPCWSTR)0xb334) {
                DAT_00550eb8 = (uint)(DAT_00550eb8 == 0);
                plVar8 = FUN_00407e50(&local_590,L"list_view_chk_sel");
                pwVar15 = (wchar_t *)(ulonglong)DAT_00550eb8;
                goto LAB_00472a5f;
              }
              if (((LPCWSTR)0xb266 < param_3) && (param_3 < (LPCWSTR)0xb28f)) {
                DAT_00546634 = 1;
                DAT_00546638 = (iVar4 * 5 + -0x37bf9) * 2;
                DAT_00546638 = FUN_004225d8(DAT_00546638,1);
                plVar8 = FUN_00407e50(&local_580,L"tab_max_width");
                FUN_004554a0(param_1,plVar8,(ulonglong)DAT_00546638);
                if (param_3 == (LPCWSTR)0xb267) {
                  DAT_00546634 = 0;
                }
                plVar8 = FUN_00407e50(&local_570,L"tab_width_mode");
                pwVar15 = (wchar_t *)(ulonglong)DAT_00546634;
                goto LAB_00472b11;
              }
              if ((param_3 < (LPCWSTR)0x848c) || ((LPCWSTR)0x848e < param_3)) {
                if ((param_3 < (LPCWSTR)0x80b1) || ((LPCWSTR)0x80b4 < param_3)) {
                  if (param_3 == (LPCWSTR)0xbd79) {
                    DAT_00546c4c = 7;
                    plVar8 = FUN_00407e50(&local_540,L"Max");
                    pwVar15 = (LPCWSTR)0x7;
                    goto LAB_00473971;
                  }
                  if (param_3 == (LPCWSTR)0xb31c) {
                    DAT_00550ee0 = (uint)(DAT_00550ee0 == 0);
                    plVar8 = FUN_00407e50(&local_530,L"enable_priv");
                    pwVar15 = (wchar_t *)(ulonglong)DAT_00550ee0;
                    FUN_004554a0(param_1,plVar8,pwVar15);
                    FUN_00457970(DAT_00550ee0);
                  }
                  else {
                    if (param_3 != (LPCWSTR)0x84cf) {
                      if ((param_3 < (LPCWSTR)0x80b5) || ((LPCWSTR)0x80b7 < param_3)) {
                        if ((param_3 < (LPCWSTR)0x8497) || ((LPCWSTR)0x84a2 < param_3)) {
                          if (param_3 == (LPCWSTR)0x800a) {
                            FUN_004122fc(local_338,1);
                            pHVar13 = GetActiveWindow();
                            pwVar15 = (LPCWSTR)0x0;
                            FUN_0040a084((longlong)local_338,pHVar13,0);
                            FUN_0040fa68((longlong)local_338);
                          }
                          else {
                            if (param_3 == (LPCWSTR)0x807a) {
                              FUN_00457438(param_1);
                              return 1;
                            }
                            if (param_3 == (LPCWSTR)0xb31f) {
                              uVar11 = FUN_0043ff60(0x551530,1,0);
                              if ((int)uVar11 == 0) {
                                return 0;
                              }
                              plVar8 = FUN_0043fee0(&local_778);
                              pwVar15 = (LPCWSTR)0x0;
                              FUN_004eff4c(&local_700,(LPCWSTR)*plVar8);
                              piVar19 = (int *)(local_778 + -0xc);
                              if (piVar19 != (int *)PTR_DAT_00546008) {
                                LOCK();
                                iVar4 = *piVar19;
                                *piVar19 = *piVar19 + -1;
                                UNLOCK();
                                if (iVar4 + -1 < 1) {
                                  thunk_FUN_0050e3a0((LPVOID)(local_778 + -0xc));
                                }
                              }
                              lVar7 = FUN_00456560(param_1);
                              if (lVar7 != 0) {
                                FUN_004045dc(&local_708,&local_6f8);
                                pWVar16 = (LPCWSTR)0x0;
                                pwVar15 = (LPCWSTR)0xffffffff;
                                local_710 = &PTR_FUN_00527d50;
                                FUN_004cebf4(lVar7,&local_710,-1,(LPCITEMIDLIST)0x0);
                              }
                              puVar9 = &local_700;
                              goto LAB_00472dc5;
                            }
                            if (param_3 == (LPCWSTR)0x800b) {
                              FUN_0044178c(0x551530);
                              return 0;
                            }
                            if (param_3 == (LPCWSTR)0x8049) {
                              FUN_0043ff60(0x551530,1,1);
                              return 1;
                            }
                            if (param_3 == (LPCWSTR)0xa770) {
                              lVar7 = FUN_00456560(param_1);
                              if (lVar7 != 0) {
                                FUN_004bf818(lVar7);
                                goto LAB_00472e9b;
                              }
                            }
                            else {
                              if (param_3 == (LPCWSTR)0x805d) {
                                if (param_4 == (LPCWSTR)0x64) {
                                  SetWindowPos(*(HWND *)(param_1 + 8),(HWND)0x0,0,0,0,0,3);
                                  SetFocus(*(HWND *)(param_1 + 8));
                                }
                                else {
                                  uVar5 = (uint)(*(int *)(param_1 + 0x4e24) == 0);
                                  *(uint *)(param_1 + 0x4e24) = uVar5;
                                  if (uVar5 == 0) {
                                    FUN_00456c18(param_1);
                                  }
                                  else {
                                    FUN_00456b14(param_1);
                                  }
                                }
LAB_00472e9b:
                                *param_5 = 1;
                                return 1;
                              }
                              if (param_3 == (LPCWSTR)0x2) {
                                if (*(int *)(param_1 + 0x4e24) != 0) {
                                  FUN_00456c18(param_1);
                                }
                              }
                              else {
                                if (param_3 == (LPCWSTR)0x8097) {
                                  uVar11 = FUN_0043ff60(0x551530,1,0);
                                  if ((int)uVar11 == 0) {
                                    return 0;
                                  }
                                  FUN_0043fee0((longlong *)&local_7c8);
                                  FUN_0040331c((longlong *)&local_780,"_");
                                  FUN_0040331c((longlong *)&local_790,"_");
                                  FUN_0040331c((longlong *)&local_7a0,"_");
                                  plVar8 = FUN_004c70b4(param_1 + 0x4e38,&local_768);
                                  local_7d8 = (undefined8 *)PTR_DAT_00546000;
                                  FUN_00402204((longlong *)&local_7d8,
                                               *(int *)((undefined8 *)*plVar8 + -1),
                                               (undefined8 *)*plVar8,*(int *)(local_7a0 + -1),
                                               local_7a0);
                                  plVar8 = FUN_004c70b4(param_1 + 0x5950,&local_748);
                                  local_7e8 = (undefined8 *)PTR_DAT_00546000;
                                  FUN_00402204((longlong *)&local_7e8,*(int *)(local_7d8 + -1),
                                               local_7d8,*(int *)((undefined8 *)*plVar8 + -1),
                                               (undefined8 *)*plVar8);
                                  local_7f8 = (undefined8 *)PTR_DAT_00546000;
                                  FUN_00402204((longlong *)&local_7f8,*(int *)(local_7e8 + -1),
                                               local_7e8,*(int *)(local_790 + -1),local_790);
                                  plVar8 = FUN_004c70b4(param_1 + 0x6468,&local_7b8);
                                  local_820 = (undefined8 *)PTR_DAT_00546000;
                                  FUN_00402204((longlong *)&local_820,*(int *)(local_7f8 + -1),
                                               local_7f8,*(int *)((undefined8 *)*plVar8 + -1),
                                               (undefined8 *)*plVar8);
                                  puVar9 = local_820;
                                  local_758 = (undefined8 *)PTR_DAT_00546000;
                                  FUN_00402204((longlong *)&local_758,*(int *)(local_820 + -1),
                                               local_820,*(int *)(local_780 + -1),local_780);
                                  plVar8 = FUN_004c70b4(param_1 + 0x6f80,&local_7b0);
                                  puVar1 = local_758;
                                  local_838 = PTR_DAT_00546000;
                                  uVar5 = *(uint *)((undefined8 *)*plVar8 + -1);
                                  pWVar16 = (LPCWSTR)(ulonglong)uVar5;
                                  FUN_00402204((longlong *)&local_838,*(int *)(local_758 + -1),
                                               local_758,uVar5,(undefined8 *)*plVar8);
                                  piVar19 = (int *)((longlong)puVar1 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    puVar9 = local_820;
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                      puVar9 = local_820;
                                    }
                                  }
                                  piVar19 = (int *)((longlong)puVar9 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  piVar19 = (int *)((longlong)local_7f8 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  piVar19 = (int *)((longlong)local_7e8 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  piVar19 = (int *)((longlong)local_7d8 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  piVar19 = (int *)(local_768 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0((LPVOID)(local_768 + -0xc));
                                    }
                                  }
                                  piVar19 = (int *)(local_748 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0((LPVOID)(local_748 + -0xc));
                                    }
                                  }
                                  piVar19 = (int *)(local_7b8 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0((LPVOID)(local_7b8 + -0xc));
                                    }
                                  }
                                  piVar19 = (int *)(local_7b0 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0((LPVOID)(local_7b0 + -0xc));
                                    }
                                  }
                                  piVar19 = (int *)((longlong)local_7a0 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  piVar19 = (int *)((longlong)local_790 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  piVar19 = (int *)((longlong)local_780 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  FUN_004028a4((ulonglong *)&local_838,L"\\",L"_");
                                  FUN_004028a4((ulonglong *)&local_838,L":",L"");
                                  pwVar15 = (wchar_t *)
                                            FUN_0040329c(&local_520,(longlong *)&local_838);
                                  plVar8 = FUN_0040329c(&local_510,(longlong *)&local_7c8);
                                  FUN_00468b3c(param_1,plVar8,(longlong *)pwVar15);
                                  *param_5 = 1;
                                  piVar19 = (int *)(local_838 + -0xc);
                                  if (piVar19 != (int *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *piVar19;
                                    *piVar19 = *piVar19 + -1;
                                    UNLOCK();
                                    if (iVar4 + -1 < 1) {
                                      thunk_FUN_0050e3a0(piVar19);
                                    }
                                  }
                                  pwVar10 = local_7c8 + -6;
                                  if (pwVar10 != (wchar_t *)PTR_DAT_00546008) {
                                    LOCK();
                                    iVar4 = *(int *)pwVar10;
                                    *(int *)pwVar10 = *(int *)pwVar10 + -1;
                                    UNLOCK();
                                    local_840 = local_7c8;
                                    if (iVar4 + -1 < 1) goto LAB_00473394;
                                  }
                                  goto LAB_00473cc4;
                                }
                                if (param_3 == (LPCWSTR)0x8070) {
                                  uVar5 = 0;
                                }
                                else if (param_3 == (LPCWSTR)0x8071) {
                                  uVar5 = 1;
                                }
                                else {
                                  if (param_3 != (LPCWSTR)0x8072) {
                                    if (param_3 == (LPCWSTR)0x808d) {
                                      *param_5 = 1;
                                      local_6c0 = 0;
                                      local_688 = PTR_DAT_00546000;
                                      local_6a0 = (void *)0x0;
                                      local_698 = 0;
                                      local_6c8 = &PTR_FUN_0052dbf8;
                                      local_680 = 0;
                                      FUN_004033e8((longlong *)&local_688,"Search");
                                      pHVar13 = GetActiveWindow();
                                      pwVar15 = (LPCWSTR)0x0;
                                      FUN_00467eac((longlong)&local_6c8,pHVar13,0);
                                      local_6c8 = &PTR_FUN_0052dbf8;
                                      piVar19 = (int *)(local_688 + -0xc);
                                      if (piVar19 != (int *)PTR_DAT_00546008) {
                                        LOCK();
                                        iVar4 = *piVar19;
                                        *piVar19 = *piVar19 + -1;
                                        UNLOCK();
                                        if (iVar4 + -1 < 1) {
                                          thunk_FUN_0050e3a0(local_688 + -0xc);
                                        }
                                      }
                                      if (local_6a0 != (void *)0x0) {
                                        FUN_0050d790(local_6a0);
                                      }
                                      goto LAB_00473cc4;
                                    }
                                    if (((LPCWSTR)0x8480 < param_3) && (param_3 < (LPCWSTR)0x8487))
                                    {
                                      DAT_00550f70 = iVar4 - 0x8481;
                                      plVar8 = FUN_00407e50(&local_500,L"title_info");
                                      pwVar15 = (wchar_t *)(ulonglong)DAT_00550f70;
                                      goto LAB_00472b11;
                                    }
                                    if ((param_3 < (LPCWSTR)0x84d5) || ((LPCWSTR)0x84da < param_3))
                                    {
                                      if ((param_3 < (LPCWSTR)0xb363) || ((LPCWSTR)0xb368 < param_3)
                                         ) {
                                        if ((param_3 < (LPCWSTR)0x8522) ||
                                           ((LPCWSTR)0x8524 < param_3)) {
                                          if ((param_3 < (LPCWSTR)0x857b) ||
                                             ((LPCWSTR)0x857d < param_3)) {
                                            if ((((param_3 != (LPCWSTR)0x84db) &&
                                                 (param_3 != (LPCWSTR)0x84dc)) &&
                                                (param_3 != (LPCWSTR)0xb378)) &&
                                               (param_3 != (LPCWSTR)0x84dd)) {
                                              if (param_3 == (LPCWSTR)0xb37a) {
                                                DAT_00546c70 = (uint)(DAT_00546c70 == 0);
                                                FUN_0041c368(DAT_00546c70);
                                                plVar8 = FUN_00407e50(&local_4c0,L"show_goto");
                                                pwVar15 = (wchar_t *)(ulonglong)DAT_00546c70;
                                              }
                                              else {
                                                if (((param_3 == (LPCWSTR)0xa777) ||
                                                    (param_3 == (LPCWSTR)0xa778)) ||
                                                   (param_3 == (LPCWSTR)0xa779)) {
                                                  DAT_00555274 = iVar4 - 0xa777;
                                                  plVar8 = FUN_00407e50(&local_430,L"lmouse_dblclk")
                                                  ;
                                                  pwVar15 = (wchar_t *)(ulonglong)DAT_00555274;
                                                  goto LAB_00472b11;
                                                }
                                                if (param_3 == (LPCWSTR)0x84eb) {
                                                  DAT_00550f28 = (uint)(DAT_00550f28 == 0);
                                                  plVar8 = FUN_00407e50(&local_4b0,
                                                                        L"mousewheel_focus");
                                                  pwVar15 = (wchar_t *)(ulonglong)DAT_00550f28;
                                                }
                                                else if (param_3 == (LPCWSTR)0x84f4) {
                                                  DAT_00547bc0 = (uint)(DAT_00547bc0 == 0);
                                                  plVar8 = FUN_00407e50(&local_4a0,
                                                                                                                                                
                                                  L"select_last_if_navigate_up");
                                                  pwVar15 = (wchar_t *)(ulonglong)DAT_00547bc0;
                                                }
                                                else if (param_3 == (LPCWSTR)0x84f5) {
                                                  DAT_00547bb8 = (uint)(DAT_00547bb8 == 0);
                                                  plVar8 = FUN_00407e50(&local_490,L"audio_click");
                                                  pwVar15 = (wchar_t *)(ulonglong)DAT_00547bb8;
                                                }
                                                else {
                                                  if (param_3 != (LPCWSTR)0xb36b) {
                                                    if ((param_3 < (LPCWSTR)0x8505) ||
                                                       ((LPCWSTR)0x8514 < param_3)) {
                                                      if (param_3 == (LPCWSTR)0x8518) {
                                                        ShowWindow(*(HWND *)(param_1 + 0x368),0);
                                                        DAT_00547eac = 0;
                                                        plVar8 = FUN_00407e50(&local_450,
                                                                              L"useTreeColor");
                                                        FUN_004554a0(param_1,plVar8,0);
                                                        FUN_004b3240(0);
                                                        ShowWindow(*(HWND *)(param_1 + 0x368),5);
                                                        DAT_00547420 = GetSysColor(5);
                                                        DAT_0054742c = GetSysColor(5);
                                                        DAT_00555278 = 0;
                                                        pwVar15 = L"useColor";
                                                        plVar8 = &local_440;
                                                        goto LAB_00473969;
                                                      }
                                                      if (param_3 == (LPCWSTR)0xe107) {
                                                        lVar7 = FUN_00456560(param_1);
                                                        if (lVar7 != 0) {
                                                          FUN_004cb790(lVar7);
                                                        }
                                                        goto LAB_00473cc4;
                                                      }
                                                      if ((param_3 < (LPCWSTR)0x8549) ||
                                                         ((LPCWSTR)0x8550 < param_3)) {
                                                        if ((((LPCWSTR)0xb342 < param_3) &&
                                                            (param_3 < (LPCWSTR)0xb34a)) ||
                                                           (param_3 == (LPCWSTR)0xb351)) {
                                                          FUN_00415420(0,0);
                                                        }
                                                      }
                                                      else {
                                                        local_770 = (HKEY)0x0;
                                                        LVar6 = RegOpenKeyExW((HKEY)
                                                  0xffffffff80000001,
                                                  L"Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\"
                                                  ,0,0xf003f,&local_770);
                                                  hKey = local_770;
                                                  if (LVar6 != 0) {
                                                    MessageBoxW(*(HWND *)(param_1 + 8),L"Error !?",
                                                                L"",0);
                                                    hKey = (HKEY)0x0;
                                                  }
                                                  local_844 = 0x80;
                                                  pWVar16 = (LPCWSTR)0x4;
                                                  if (iVar4 == 0x8549) {
                                                    local_844 = 0x30;
                                                  }
                                                  else if (iVar4 == 0x854a) {
                                                    local_844 = 0x40;
                                                  }
                                                  else if (iVar4 == 0x854b) {
                                                    local_844 = 0x60;
                                                  }
                                                  else if (iVar4 != 0x854c) {
                                                    if (iVar4 == 0x854d) {
                                                      local_844 = 0xa0;
                                                    }
                                                    else if (iVar4 == 0x854e) {
                                                      local_844 = 0xc0;
                                                    }
                                                    else if (iVar4 == 0x854f) {
                                                      local_844 = 0xe0;
                                                    }
                                                    else {
                                                      local_844 = 0x80;
                                                      if (iVar4 == 0x8550) {
                                                        local_844 = 0x100;
                                                      }
                                                    }
                                                  }
                                                  pwVar15 = (LPCWSTR)0x0;
                                                  RegSetValueExW(hKey,L"ThumbnailSize",0,4,
                                                                 (BYTE *)&local_844,4);
                                                  if (hKey != (HKEY)0x0) {
                                                    RegCloseKey(hKey);
                                                  }
                                                  }
                                                  }
                                                  else {
                                                    ShowWindow(*(HWND *)(param_1 + 0x368),0);
                                                    FUN_004b3240(iVar4 + -0x8505);
                                                    DAT_00547eac = 1;
                                                    DAT_00555278 = 1;
                                                    plVar8 = FUN_00407e50(&local_470,L"useTreeColor"
                                                                         );
                                                    FUN_004554a0(param_1,plVar8,1);
                                                    plVar8 = FUN_00407e50(&local_460,L"useColor");
                                                    pwVar15 = (wchar_t *)(ulonglong)DAT_00555278;
                                                    FUN_004554a0(param_1,plVar8,pwVar15);
                                                    ShowWindow(*(HWND *)(param_1 + 0x368),5);
                                                  }
                                                  goto LAB_00473b2b;
                                                  }
                                                  DAT_00550e6c = (uint)(DAT_00550e6c == 0);
                                                  plVar8 = FUN_00407e50(&local_480,
                                                                                                                                                
                                                  L"use_system_navigation_click");
                                                  pwVar15 = (wchar_t *)(ulonglong)DAT_00550e6c;
                                                }
                                              }
                                              goto LAB_00473971;
                                            }
                                            FUN_00463bec(param_1,iVar4);
                                            goto LAB_00473b2b;
                                          }
                                          DAT_00555280 = iVar4 - 0x857b;
                                          plVar8 = FUN_00407e50(&local_4d0,
                                                                L"status_no_selection_type");
                                          pwVar15 = (wchar_t *)(ulonglong)DAT_00555280;
                                        }
                                        else {
                                          DAT_0055527c = iVar4 - 0x8522;
                                          plVar8 = FUN_00407e50(&local_4e0,L"status_selection_type")
                                          ;
                                          pwVar15 = (wchar_t *)(ulonglong)DAT_0055527c;
                                        }
LAB_00472b11:
                                        FUN_004554a0(param_1,plVar8,pwVar15);
                                      }
                                      else {
                                        if (param_3 == (LPCWSTR)0xb364) {
                                          DAT_00555a58 = 9;
                                        }
                                        else if (param_3 == (LPCWSTR)0xb365) {
                                          DAT_00555a58 = 0xc;
                                        }
                                        else if (param_3 == (LPCWSTR)0xb366) {
                                          DAT_00555a58 = 0x12;
                                        }
                                        else if (param_3 == (LPCWSTR)0xb367) {
                                          DAT_00555a58 = 0x18;
                                        }
                                        else {
                                          DAT_00555a58 = 0;
                                          if (param_3 == (LPCWSTR)0xb368) {
                                            DAT_00555a58 = 0x20;
                                          }
                                        }
                                        plVar8 = FUN_00407e50(&local_4f0,L"tree_indent_width");
                                        pwVar15 = (wchar_t *)(ulonglong)DAT_00555a58;
                                        FUN_004554a0(param_1,plVar8,pwVar15);
                                        if (*(longlong *)(param_1 + 0xa20) != 0) {
                                          FUN_004f51c8(param_1 + 0xa18);
                                        }
                                        FUN_004f51c8(param_1 + 0x52a8);
                                        FUN_004f51c8(param_1 + 24000);
                                        FUN_004f51c8(param_1 + 0x68d8);
                                        FUN_004f51c8(param_1 + 0x73f0);
                                      }
                                    }
                                    else {
                                      pwVar15 = (LPCWSTR)0x0;
                                      FUN_0045ab08(param_1,iVar4 - 0x84d5,0);
                                    }
                                    goto LAB_00473b2b;
                                  }
                                  uVar5 = 2;
                                }
                                FUN_00467b10(param_1,uVar5);
                                ShowWindow(*(HWND *)(param_1 + 0xc98),0);
                                ShowWindow(*(HWND *)(param_1 + 0xc98),5);
                              }
                              *param_5 = 1;
                            }
                          }
                          goto LAB_00473cc4;
                        }
                      }
                      else {
                        FUN_00455b8c(*(HWND *)(param_1 + 8),(longlong)param_3);
                      }
                      goto LAB_00473b2b;
                    }
                    uVar11 = FUN_004e2928(-1);
                    FUN_004e2928((uint)((int)uVar11 == 0));
                  }
                  goto LAB_00473cc4;
                }
                DAT_00546c4c = iVar4 - 0x80b1;
                plVar8 = FUN_00407e50(&local_560,L"Max");
                pwVar15 = (wchar_t *)(ulonglong)DAT_00546c4c;
                FUN_004554a0(param_1,plVar8,pwVar15);
                if (param_3 == (LPCWSTR)0x80b3) {
                  plVar8 = FUN_0040331c(&local_550,"WinRCcur");
                  pwVar15 = (LPCWSTR)0x0;
                  FUN_00457d24(param_1,plVar8,(longlong *)0x0);
                }
                *param_5 = 1;
              }
LAB_00473b2b:
              if (param_3 == (LPCWSTR)0xb352) {
                DAT_00550ea0 = (uint)(DAT_00550ea0 == 0);
                plVar8 = FUN_00407e50(&local_420,L"default_new_tab");
                pwVar15 = (wchar_t *)(ulonglong)DAT_00550ea0;
              }
              else {
                if (param_3 != (LPCWSTR)0xb37e) {
                  if (param_3 == (LPCWSTR)0xb40a) {
                    pwVar15 = (wchar_t *)(ulonglong)(DAT_00551280 == 0);
                    pWVar16 = (LPCWSTR)0x0;
                    DAT_00551280 = (uint)(DAT_00551280 == 0);
                    FUN_0047d8e4(DAT_00554108,(LPCWSTR)&DAT_00551288,pwVar15,(LPCWSTR)0x0);
                  }
                  else if (param_3 == (LPCWSTR)0xb37c) {
                    iVar4 = FUN_00407b68();
                    if (iVar4 == 0) {
                      GetModuleFileNameW(DAT_005556d0,local_268,0x104);
                      FUN_00407e50((longlong *)&local_730,L"AutoStart2017_IZASSHJ");
                      pwVar15 = local_268;
                      pWVar16 = local_730;
                      ShellExecuteW((HWND)0x0,L"runas",pwVar15,local_730,(LPCWSTR)0x0,5);
LAB_00473ca4:
                      local_840 = local_730 + -6;
                      if (local_840 != (wchar_t *)PTR_DAT_00546008) {
                        LOCK();
                        iVar4 = *(int *)local_840;
                        *(int *)local_840 = *(int *)local_840 + -1;
                        UNLOCK();
                        if (iVar4 + -1 < 1) goto LAB_00473cbf;
                      }
                    }
                    else {
                      FUN_0045a624();
                    }
                  }
                  else if (param_3 == (LPCWSTR)0xb37d) {
                    FUN_0040329c((longlong *)&local_718,&DAT_00554358);
                    uVar11 = FUN_0050c774(local_718,0);
                    local_730 = local_718;
                    if ((int)uVar11 == 0) {
                      pHVar14 = GetModuleHandleW((LPCWSTR)0x0);
                      pwVar15 = (LPCWSTR)0x0;
                      FUN_0050c934(pHVar14,local_718,0);
                    }
                    else {
                      FUN_0050c710(local_718,0);
                    }
                    goto LAB_00473ca4;
                  }
                  goto LAB_00473cc4;
                }
                DAT_005541c0 = (uint)(DAT_005541c0 == 0);
                plVar8 = FUN_00407e50(&local_410,L"move_file_confirmation");
                pwVar15 = (wchar_t *)(ulonglong)DAT_005541c0;
              }
            }
            goto LAB_00473971;
          }
          FUN_00467de0(param_1);
        }
        goto LAB_00473cc4;
      }
      DAT_00550ec0 = (uint)(DAT_00550ec0 == 0);
      plVar8 = FUN_00407e50(&local_5c0,L"list_disable_transparent_sel");
      uVar5 = DAT_00550ec0;
LAB_00472973:
      pwVar15 = (wchar_t *)(ulonglong)uVar5;
      FUN_004554a0(param_1,plVar8,pwVar15);
      local_848 = 1;
    }
  }
LAB_00471ff7:
  FUN_004bfae4(param_1 + 0x4e38,0);
  FUN_004bfae4(param_1 + 0x5950,0);
  FUN_004bfae4(param_1 + 0x6468,0);
  puVar12 = (ulonglong *)0x0;
  FUN_004bfae4(param_1 + 0x6f80,0);
  if (local_848 == 2) {
    puVar12 = FUN_004a3f04(&local_400,0xbe,pwVar15,(ulonglong)pWVar16);
    FUN_004c00ac(param_1 + 0x4e38,(longlong *)puVar12);
    puVar12 = FUN_004a3f04(&local_3f0,0xbe,pwVar15,(ulonglong)pWVar16);
    FUN_004c00ac(param_1 + 0x5950,(longlong *)puVar12);
    puVar12 = FUN_004a3f04(&local_3e0,0xbe,pwVar15,(ulonglong)pWVar16);
    FUN_004c00ac(param_1 + 0x6468,(longlong *)puVar12);
    puVar12 = FUN_004a3f04(&local_3d0,0xbe,pwVar15,(ulonglong)pWVar16);
    FUN_004c00ac(param_1 + 0x6f80,(longlong *)puVar12);
  }
  if (local_848 != 3) {
    return 0;
  }
LAB_004720c7:
  FUN_00455d20(param_1,puVar12,pwVar15,(ulonglong)pWVar16);
  return 0;
}


