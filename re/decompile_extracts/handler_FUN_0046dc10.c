// ==== FUN_0046dc10 @ 0046dc10 ====

undefined8
FUN_0046dc10(longlong param_1,undefined8 param_2,short param_3,undefined8 param_4,
            LPCITEMIDLIST param_5)

{
  BOOL BVar1;
  uint uVar2;
  longlong *plVar3;
  longlong lVar4;
  bool bVar5;
  longlong local_48 [3];
  
  *(undefined4 *)param_5 = 1;
  if (param_3 == -0x7f94) {
    DAT_00550f68 = 1;
    plVar3 = FUN_00407e50(local_48,L"tree_mode");
    FUN_004554a0(param_1,plVar3,1);
    DAT_00555270 = 0;
    FUN_004ceb00((longlong *)(param_1 + 0x4e38));
    FUN_004ceb00((longlong *)(param_1 + 0x5950));
    FUN_004ceb00((longlong *)(param_1 + 0x6468));
    FUN_004ceb00((longlong *)(param_1 + 0x6f80));
    if (*(int *)(param_1 + 0x3f4) != -1) {
      ShowWindow(*(HWND *)(param_1 + 0x3a0 + (ulonglong)(*(int *)(param_1 + 0x3f4) == 0) * 8),5);
    }
    *(undefined4 *)(param_1 + 0x3f4) = 0xffffffff;
    FUN_0042a228((undefined8 *)(param_1 + 0x3a0));
    *(undefined4 *)(param_1 + 0x3dc) = *(undefined4 *)(param_1 + 0x7b14);
    SetParent(*(HWND *)(param_1 + 0x7aa0),*(HWND *)(param_1 + 0x368));
    lVar4 = FUN_00456560(param_1);
    if (*(longlong *)(param_1 + 0x410) != 0) {
      FUN_004532c8(param_1 + 0x408);
    }
    param_5 = (LPCITEMIDLIST)0x0;
    SHGetSpecialFolderLocation((HWND)0x0,0,&param_5);
    if (lVar4 != 0) {
      FUN_004c6fbc(lVar4,local_48);
      FUN_004efeb4(local_48);
    }
    if (*(longlong *)(param_1 + 0x410) != 0) {
      FUN_004532a8(param_1 + 0x408,param_5);
    }
    if (*(longlong *)(param_1 + 0x368) != 0) {
      if (*(int *)(param_1 + 1000) == 0) {
        FUN_0042a3bc((undefined8 *)(param_1 + 0x3a0),0x96,'\x01');
      }
      else {
        FUN_0042a4dc((undefined8 *)(param_1 + 0x3a0),(undefined8 *)0x0,'\x01');
      }
    }
    FUN_00465054(param_1,1);
  }
  else if (param_3 == -0x7f93) {
    DAT_00550f68 = 2;
    plVar3 = FUN_00407e50((longlong *)&param_5,L"tree_mode");
    FUN_004554a0(param_1,plVar3,2);
    DAT_00555270 = 1;
    ShowWindow(*(HWND *)(param_1 + 0x7aa0),0);
    FUN_004ceb00((longlong *)(param_1 + 0x4e38));
    FUN_004ceb00((longlong *)(param_1 + 0x5950));
    FUN_004ceb00((longlong *)(param_1 + 0x6468));
    FUN_004ceb00((longlong *)(param_1 + 0x6f80));
    BVar1 = IsWindowVisible(*(HWND *)(param_1 + 0x3a8));
    if (BVar1 == 0) {
      ShowWindow(*(HWND *)(param_1 + 0x3a8),5);
    }
    ShowWindow(*(HWND *)(param_1 + 0x3a0),0);
    if (*(int *)(param_1 + 0x3c8) != 1) {
      *(undefined4 *)(param_1 + 0x3c8) = 1;
    }
    *(undefined4 *)(param_1 + 0x3f4) = 1;
    FUN_0042a228((undefined8 *)(param_1 + 0x3a0));
    if (*(longlong *)(param_1 + 0x410) != 0) {
      FUN_004532c8(param_1 + 0x408);
    }
    ShowWindow(*(HWND *)(param_1 + 0x7aa0),5);
  }
  else {
    if (param_3 != -0x7f95) {
      if (param_3 == -0x7f96) {
        DAT_00550f68 = DAT_00550f68 + 1;
        FUN_00456d78(param_1);
        return 1;
      }
      if (2 < (ushort)(param_3 - 0x413U)) {
        return 1;
      }
      uVar2 = DAT_00547eac;
      if (param_3 == 0x413) {
        uVar2 = 0;
      }
      if (param_3 == 0x414) {
        uVar2 = 1;
      }
      if (param_3 == 0x415) {
        uVar2 = 2;
      }
      bVar5 = DAT_00547eac != uVar2;
      DAT_00547eac = uVar2;
      if (bVar5) {
        FUN_004f1648();
        DAT_00547428 = DAT_00547428 + 1;
      }
      if (*(longlong *)(param_1 + 0x410) != 0) {
        FUN_004f51c8(param_1 + 0xa18);
      }
      FUN_004f51c8(param_1 + 0x52a8);
      FUN_004f51c8(param_1 + 24000);
      FUN_004f51c8(param_1 + 0x68d8);
      FUN_004f51c8(param_1 + 0x73f0);
      plVar3 = FUN_00407e50((longlong *)&param_5,L"useTreeColor");
      FUN_004554a0(param_1,plVar3,(ulonglong)DAT_00547eac);
      return 1;
    }
    DAT_00550f68 = 0;
    plVar3 = FUN_00407e50((longlong *)&param_5,L"tree_mode");
    FUN_004554a0(param_1,plVar3,0);
    DAT_00555270 = 0;
    FUN_004ceb00((longlong *)(param_1 + 0x4e38));
    FUN_004ceb00((longlong *)(param_1 + 0x5950));
    FUN_004ceb00((longlong *)(param_1 + 0x6468));
    FUN_004ceb00((longlong *)(param_1 + 0x6f80));
    BVar1 = IsWindowVisible(*(HWND *)(param_1 + 0x3a8));
    if (BVar1 == 0) {
      ShowWindow(*(HWND *)(param_1 + 0x3a8),5);
    }
    ShowWindow(*(HWND *)(param_1 + 0x3a0),0);
    if (*(int *)(param_1 + 0x3c8) != 1) {
      *(undefined4 *)(param_1 + 0x3c8) = 1;
    }
    *(undefined4 *)(param_1 + 0x3f4) = 1;
    FUN_0042a228((undefined8 *)(param_1 + 0x3a0));
    if (*(longlong *)(param_1 + 0x410) != 0) {
      FUN_004532c8(param_1 + 0x408);
    }
  }
  FUN_0045e064(param_1);
  FUN_00419f6c(param_1 + 0xa8,DAT_00550f68);
  return 1;
}


