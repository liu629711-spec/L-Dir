// ==== FUN_004626b8 @ 004626b8 ====

LRESULT FUN_004626b8(longlong param_1,undefined8 param_2,short param_3)

{
  int iVar1;
  ulonglong *puVar2;
  longlong *plVar3;
  LRESULT LVar4;
  int *piVar5;
  undefined *local_38;
  ulonglong local_30;
  longlong local_28;
  longlong local_20;
  
  if (param_3 == -0x428b) {
    puVar2 = FUN_00461840(&local_30);
    local_38 = PTR_DAT_00546000;
    iVar1 = lstrlenW(L"\\start2.qdr");
    FUN_00402204((longlong *)&local_38,*(int *)((undefined8 *)*puVar2 + -1),(undefined8 *)*puVar2,
                 iVar1,(undefined8 *)L"\\start2.qdr");
    piVar5 = (int *)(local_30 - 0xc);
    if (piVar5 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar5;
      *piVar5 = *piVar5 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_30 - 0xc));
      }
    }
    plVar3 = FUN_0040329c(&local_28,(longlong *)&local_38);
    FUN_0045cf90(param_1,plVar3,1,1,0,0,0);
    plVar3 = FUN_0040329c(&local_20,(longlong *)&local_38);
    FUN_004764a0(1,plVar3);
    piVar5 = (int *)(local_38 + -0xc);
    if (piVar5 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar5;
      *piVar5 = *piVar5 + -1;
      UNLOCK();
      if (iVar1 == 1 || iVar1 + -1 < 0) {
        thunk_FUN_0050e3a0(piVar5);
      }
    }
    LVar4 = 1;
  }
  else {
    if (param_3 == -0x4289) {
      DAT_00550fd0 = 2;
    }
    SendMessageW(*(HWND *)(param_1 + 8),0x111,0xe100,0);
    LVar4 = SendMessageW(*(HWND *)(param_1 + 8),0x111,0xe141,0);
  }
  return LVar4;
}


