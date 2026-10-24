// ==== FUN_00461e54 @ 00461e54 ====

int FUN_00461e54(longlong param_1)

{
  undefined8 *puVar1;
  int iVar2;
  longlong *plVar3;
  int *piVar4;
  ulonglong local_res10;
  undefined8 *local_res18;
  longlong local_res20;
  longlong local_28;
  longlong local_20;
  
  EnterCriticalSection((LPCRITICAL_SECTION)(DAT_00549b88 + 0x60));
  if (DAT_00550f84 == 0) {
    DAT_00550f84 = 1;
    if (DAT_00550e58 == 0) {
      plVar3 = FUN_00407e50(&local_28,L"Als");
      iVar2 = FUN_0045554c(param_1,plVar3,5);
      if (iVar2 == 5) {
        FUN_00461840(&local_res10);
        puVar1 = DAT_00551028;
        local_res18 = (undefined8 *)PTR_DAT_00546000;
        iVar2 = lstrlenW(L"\\");
        FUN_00402204((longlong *)&local_res18,iVar2,(undefined8 *)&DAT_00528314,
                     *(int *)(puVar1 + -1),DAT_00551028);
        puVar1 = local_res18;
        FUN_004022a4((longlong *)&local_res10,*(int *)(local_res18 + -1),local_res18);
        piVar4 = (int *)((longlong)puVar1 + -0xc);
        if (piVar4 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar2 = *piVar4;
          *piVar4 = *piVar4 + -1;
          UNLOCK();
          if (iVar2 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar4);
          }
        }
        plVar3 = FUN_0040329c(&local_20,(longlong *)&local_res10);
        FUN_0045cf90(param_1,plVar3,1,1,1,1,0);
        piVar4 = (int *)(local_res10 - 0xc);
        if (piVar4 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar2 = *piVar4;
          *piVar4 = *piVar4 + -1;
          UNLOCK();
          if (iVar2 + -1 < 1) {
            thunk_FUN_0050e3a0((LPVOID)(local_res10 - 0xc));
          }
        }
      }
    }
    else {
      plVar3 = FUN_0040329c(&local_res20,(longlong *)&DAT_00551028);
      FUN_0045cf90(param_1,plVar3,1,1,1,1,0);
    }
  }
  LeaveCriticalSection((LPCRITICAL_SECTION)(DAT_00549b88 + 0x60));
  return DAT_00550f84;
}


