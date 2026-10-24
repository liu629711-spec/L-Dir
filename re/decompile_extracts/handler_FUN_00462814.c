// ==== FUN_00462814 @ 00462814 ====

undefined8 FUN_00462814(longlong param_1)

{
  int iVar1;
  ulonglong *puVar2;
  longlong *plVar3;
  int *piVar4;
  undefined *local_38;
  ulonglong local_30;
  longlong local_28;
  longlong local_20;
  
  puVar2 = FUN_00461840(&local_30);
  local_38 = PTR_DAT_00546000;
  iVar1 = lstrlenW(L"\\start2.qdr");
  FUN_00402204((longlong *)&local_38,*(int *)((undefined8 *)*puVar2 + -1),(undefined8 *)*puVar2,
               iVar1,(undefined8 *)L"\\start2.qdr");
  piVar4 = (int *)(local_30 - 0xc);
  if (piVar4 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar4;
    *piVar4 = *piVar4 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0((LPVOID)(local_30 - 0xc));
    }
  }
  plVar3 = FUN_0040329c(&local_28,(longlong *)&local_38);
  FUN_0045cf90(param_1,plVar3,1,1,0,0,0);
  plVar3 = FUN_0040329c(&local_20,(longlong *)&local_38);
  FUN_0045a1e0(param_1,plVar3);
  piVar4 = (int *)(local_38 + -0xc);
  if (piVar4 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar4;
    *piVar4 = *piVar4 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0(piVar4);
    }
  }
  return 0;
}


