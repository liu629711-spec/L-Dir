// ==== FUN_00467268 @ 00467268 ====

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined8
FUN_00467268(longlong param_1,undefined8 param_2,ushort param_3,ulonglong param_4,
            undefined4 *param_5)

{
  int iVar1;
  uint uVar2;
  longlong lVar3;
  longlong *plVar4;
  ulonglong *puVar5;
  HWND pHVar6;
  HKEY hKey;
  HWND hWnd;
  undefined8 uVar7;
  ulonglong uVar8;
  int *piVar9;
  SHELLFLAGSTATE local_bc8 [2];
  HKEY local_bc0;
  HKEY local_bb8;
  ulonglong local_bb0;
  SHELLSTATEA local_ba8;
  SHELLSTATEA local_b88;
  SHELLSTATEA local_b68;
  SHELLSTATEA local_b48;
  longlong local_b28;
  longlong local_b20;
  longlong local_b18;
  longlong local_b10;
  longlong local_b08;
  longlong local_b00;
  longlong local_af8;
  ulonglong local_af0;
  longlong local_ae8;
  SHELLSTATEA local_ae0;
  longlong local_ac0;
  longlong local_ab8 [2];
  undefined8 local_aa8;
  HWND local_aa0;
  undefined4 local_a38;
  WCHAR local_b8 [64];
  undefined8 local_38;
  
  local_38 = DAT_00547f48;
  uVar7 = 0x78;
  FUN_0050dc70((undefined8 *)local_b8,
               (undefined8 *)L"Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
               0x78);
  *param_5 = 1;
  if (param_3 == 0x8504) {
LAB_004676c1:
    puVar5 = FUN_004a3f04(&local_bb0,0x3a,uVar7,param_4);
    local_af0 = *puVar5;
    FUN_004b2494(&local_aa8,local_af0,0,(HPROPSHEETPAGE)0x0);
    piVar9 = (int *)(local_bb0 - 0xc);
    if (piVar9 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar9;
      *piVar9 = *piVar9 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_bb0 - 0xc));
      }
    }
    if (param_3 == 0x8504) {
      if (local_aa0 == (HWND)0x0) {
        local_a38 = 1;
      }
      else {
        SendMessageW(local_aa0,0x465,1,0);
      }
    }
    pHVar6 = *(HWND *)(param_1 + 8);
    do {
      hWnd = pHVar6;
      pHVar6 = GetParent(hWnd);
    } while (pHVar6 != (HWND)0x0);
    FUN_004613b0((longlong)&local_aa8,hWnd);
    FUN_004ae908(&local_aa8);
    return 0;
  }
  if (0x13 < (ushort)(param_3 + 0x7fe7)) {
    return 0;
  }
  if (0x801b < param_3) {
    if (param_3 == 0x802b) {
      DAT_00547bb0 = (uint)(DAT_00547bb0 == 0);
      plVar4 = FUN_00407e50(&local_ae8,L"merke_ansicht");
      uVar2 = DAT_00547bb0;
    }
    else {
      if (param_3 != 0x802c) {
        if (param_3 == 0x801e) {
          ShellExecuteW(*(HWND *)(param_1 + 8),L"open",L"control.exe",L"folders",(LPCWSTR)0x0,5);
          return 1;
        }
        if (param_3 != 0x801f) {
          if (param_3 == 0x8020) {
            plVar4 = FUN_0040331c(&local_b10,"Folder");
            uVar7 = FUN_00462960(plVar4,0,1);
            plVar4 = FUN_0040331c(&local_b00,"Folder");
            FUN_00462960(plVar4,(uint)((int)uVar7 == 0),1);
            return 0;
          }
          if (param_3 == 0x8024) goto LAB_004676c1;
          if (param_3 == 0x8022) {
            DAT_00555278 = (uint)(DAT_00555278 == 0);
            plVar4 = FUN_00407e50(&local_b18,L"useColor");
            uVar8 = (ulonglong)DAT_00555278;
          }
          else {
            if (param_3 == 0x8021) {
              return 0;
            }
            if (param_3 == 0x8023) {
              DAT_00547bbc = (uint)(DAT_00547bbc == 0);
              plVar4 = FUN_00407e50(&local_b28,L"spalten_speichern");
              uVar8 = (ulonglong)DAT_00547bbc;
            }
            else if (param_3 == 0x8025) {
              DAT_0055528c = (uint)(DAT_0055528c == 0);
              plVar4 = FUN_00407e50(&local_b08,L"fullrow_sel");
              uVar8 = (ulonglong)DAT_0055528c;
            }
            else if (param_3 == 0x8026) {
              DAT_00555284 = (uint)(DAT_00555284 == 0);
              plVar4 = FUN_00407e50(local_ab8,L"show_linie");
              uVar8 = (ulonglong)DAT_00555284;
            }
            else {
              if (param_3 == 0x8029) {
                DAT_00547b90 = (uint)(DAT_00547b90 == 0);
                plVar4 = FUN_00407e50(&local_af8,L"adress_tree");
                FUN_004554a0(param_1,plVar4,(ulonglong)DAT_00547b90);
                *(uint *)(param_1 + 0x1a0) = DAT_00547b90;
                goto LAB_00467900;
              }
              if (param_3 != 0x802a) {
                return 0;
              }
              DAT_00547ba8 = (uint)(DAT_00547ba8 == 0);
              plVar4 = FUN_00407e50(&local_ac0,L"adress_autosel");
              uVar8 = (ulonglong)DAT_00547ba8;
            }
          }
          FUN_004554a0(param_1,plVar4,uVar8);
        }
LAB_00467900:
        FUN_004bfae4(param_1 + 0x4e38,0);
        FUN_004bfae4(param_1 + 0x5950,0);
        FUN_004bfae4(param_1 + 0x6468,0);
        FUN_004bfae4(param_1 + 0x6f80,0);
        return 0;
      }
      DAT_00547bac = (uint)(DAT_00547bac == 0);
      plVar4 = FUN_00407e50(&local_b20,L"merke_markierte");
      uVar2 = DAT_00547bac;
    }
    FUN_004554a0(param_1,plVar4,(ulonglong)uVar2);
    return 0;
  }
  SHGetSettings(local_bc8,0x42023);
  if (param_3 == 0x8019) {
    local_ba8._0_4_ = 0;
    FUN_0050daf0((ulonglong *)&local_ba8.dwWin95Unused,0,0x1c);
    SHGetSetSettings(&local_ba8,0x40021,0);
    uVar2 = ~local_ba8.fShowAllObjects;
    local_ba8.fShowSuperHidden = (uVar2 & 1) == 0;
    local_ba8.fShowSysFiles = uVar2;
    local_ba8.fShowAllObjects = uVar2;
    SHGetSetSettings(&local_ba8,0x40021,1);
    if (SPULL(local_bc8[0],0,1) == 0) {
      local_b88._0_4_ = local_b88._0_4_ | 9;
    }
    else {
      local_b88._0_4_ = local_b88._0_4_ & 0xfffffff6;
    }
    SHGetSetSettings(&local_b88,0x21,1);
    lVar3 = FUN_00456560(param_1);
    if (lVar3 != 0) {
      FUN_004c0748(lVar3);
    }
    local_ae0._0_4_ = 0;
    FUN_0050daf0((ulonglong *)&local_ae0.dwWin95Unused,0,0x1c);
    SHGetSetSettings(&local_ae0,0x40021,0);
    if (((uint)(local_ae0._0_4_ ^ local_ba8._0_4_) >> 0xf & 1) != 0) goto LAB_0046757f;
    local_bc0 = (HKEY)0x0;
    FUN_004065d8(&local_bc0,(HKEY)0xffffffff80000001,local_b8,(LPWSTR)0x0,0,0xf003f,
                 (LPSECURITY_ATTRIBUTES)0x0,(DWORD *)0x0);
    FUN_00454e6c(&local_bc0,1,L"ShowSuperHidden");
    FUN_00454e6c(&local_bc0,0,L"SuperHidden");
    lVar3 = FUN_00456560(param_1);
    hKey = local_bc0;
    if (lVar3 != 0) {
      FUN_004c0748(lVar3);
      hKey = local_bc0;
    }
  }
  else {
    if (param_3 == 0x801a) {
      if (SPULL(local_bc8[0],1,1) == 0) {
        local_b48._0_4_ = local_b48._0_4_ | 2;
      }
      else {
        local_b48._0_4_ = local_b48._0_4_ & 0xfffffffd;
      }
      SHGetSetSettings(&local_b48,2,1);
      goto LAB_0046757f;
    }
    if (param_3 != 0x801b) goto LAB_0046757f;
    local_b68._0_4_ = 0;
    FUN_0050daf0((ulonglong *)&local_b68.dwWin95Unused,0,0x1c);
    SHGetSetSettings(&local_b68,0x40021,0);
    local_bb8 = (HKEY)0x0;
    FUN_004065d8(&local_bb8,(HKEY)0xffffffff80000001,local_b8,(LPWSTR)0x0,0,0xf003f,
                 (LPSECURITY_ATTRIBUTES)0x0,(DWORD *)0x0);
    FUN_00454e6c(&local_bb8,(~local_b68._0_4_ & 0x8000) >> 0xf,L"ShowSuperHidden");
    FUN_00454e6c(&local_bb8,(local_b68._0_4_ & 0x8000) >> 0xf,L"SuperHidden");
    lVar3 = FUN_00456560(param_1);
    hKey = local_bb8;
    if (lVar3 != 0) {
      FUN_004c0748(lVar3);
      hKey = local_bb8;
    }
  }
  if (hKey != (HKEY)0x0) {
    RegCloseKey(hKey);
  }
LAB_0046757f:
  lVar3 = FUN_00456560(param_1);
  if (lVar3 == 0) {
    return 0;
  }
  FUN_004bf818(lVar3);
  *param_5 = 1;
  return 1;
}


