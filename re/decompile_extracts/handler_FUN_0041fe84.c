// ==== FUN_0041fe84 @ 0041fe84 ====

longlong FUN_0041fe84(longlong param_1,undefined8 param_2,ulonglong param_3,LPARAM param_4,
                     undefined4 *param_5)

{
  int iVar1;
  longlong lVar2;
  LRESULT LVar3;
  HWND hWnd;
  int local_res8 [8];
  LPCWSTR local_48;
  undefined8 local_40;
  WPARAM local_38;
  
  if (((*(int *)(param_1 + 0x48) != 0) && (param_3 - 0xb28f < 0x8c)) &&
     (*(longlong *)(param_1 + 0x118) != 0)) {
    FUN_0040331c((longlong *)&local_48,"");
    if ((*(longlong *)(param_1 + 0x60) != 0) &&
       (lVar2 = FUN_00427c64(*(longlong *)(param_1 + 0x60),(int)param_3,local_res8), lVar2 != 0)) {
      FUN_00402154((longlong *)&local_48,(longlong *)(lVar2 + 0x78));
      iVar1 = lstrlenW(L":\\");
      FUN_004022a4((longlong *)&local_48,iVar1,(undefined8 *)&DAT_00529760);
    }
    FUN_004eff4c(&local_40,local_48);
    if (local_38 != 0) {
      LVar3 = SendMessageW(*(HWND *)(param_1 + 0x118),0x4e0,local_38,*(LPARAM *)(param_1 + 8));
      lVar2 = (longlong)(int)LVar3;
      FUN_004efeb4(&local_40);
      local_48 = local_48 + -6;
      if (local_48 == (LPCWSTR)PTR_DAT_00546008) {
        return lVar2;
      }
      LOCK();
      iVar1 = *(int *)local_48;
      *(int *)local_48 = *(int *)local_48 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0(local_48);
        return lVar2;
      }
      return lVar2;
    }
    FUN_004efeb4(&local_40);
    local_48 = local_48 + -6;
    if (local_48 != (LPCWSTR)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *(int *)local_48;
      *(int *)local_48 = *(int *)local_48 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0(local_48);
      }
    }
  }
  if (param_3 < 0xa774) {
    if (param_3 != 0xa773) {
      if ((param_3 < 0x6f) ||
         ((0x70 < param_3 &&
          ((param_3 < 0xa76c || ((0xa76d < param_3 && ((param_3 < 0xa770 || (0xa772 < param_3)))))))
          ))) goto LAB_0042007c;
      goto LAB_0041fffd;
    }
    if (*(int *)(param_1 + 0x4c) == 0) {
      param_3 = 0x806c;
LAB_00420047:
      param_4 = 0;
      hWnd = DAT_00550f60;
      goto LAB_0042000f;
    }
    if (*(int *)(param_1 + 0x4c) == 1) {
      param_3 = 0x806b;
      goto LAB_00420047;
    }
    hWnd = *(HWND *)(param_1 + 0x118);
    param_3 = 0xa773;
  }
  else {
    if (param_3 != 0xa775) {
      if (param_3 == 0xb379) {
        if (*(HWND *)(param_1 + 0x118) == (HWND)0x0) {
          return 1;
        }
        if ((*(int *)(param_1 + 0x15c) != 0) && (*(longlong *)(param_1 + 0x108) != 0)) {
          FUN_00419db4(*(longlong *)(param_1 + 0x108));
          return 1;
        }
        LVar3 = SendMessageW(*(HWND *)(param_1 + 0x118),0xd20,0,*(LPARAM *)(param_1 + 0xa0));
        return (longlong)(int)LVar3;
      }
      if (param_3 == 0x1e240) {
        FUN_0041f358(param_1);
        MessageBoxW(*(HWND *)(param_1 + 8),L"§§",L"",0);
        return 1;
      }
LAB_0042007c:
      *param_5 = 0;
      return 0;
    }
LAB_0041fffd:
    hWnd = *(HWND *)(param_1 + 0x118);
  }
  if (hWnd == (HWND)0x0) {
    return 1;
  }
LAB_0042000f:
  SendMessageW(hWnd,0x111,param_3,param_4);
  return 1;
}


